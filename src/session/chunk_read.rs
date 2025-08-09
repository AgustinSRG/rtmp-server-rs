// Chunk read logic

use std::{cmp, time::Duration};

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use chrono::Utc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    log_debug, log_error,
    rtmp::{
        get_rtmp_header_size, rtmp_make_ack, RTMP_CHUNK_TYPE_0, RTMP_CHUNK_TYPE_1,
        RTMP_CHUNK_TYPE_2, RTMP_PING_TIMEOUT, RTMP_TYPE_METADATA,
    },
    server::RtmpServerContext,
};

use super::{
    handle_rtmp_packet, session_write_bytes, RtmpPacketWrapper, SessionReadThreadContext,
    IN_PACKETS_BUFFER_SIZE,
};

/// Interval to compute bit rate (milliseconds)
const BIT_RATE_COMPUTE_INTERVAL_MS: i64 = 1000;

/// Reads a RTMP chunk
/// Handles the packet when the last chunk of the packet is read
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `read_stream` - The stream to read from the client
/// * `write_stream` - The stream to write to the client
/// * `in_packets` - Array of input packets
pub async fn read_rtmp_chunk<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    read_stream: &mut TR,
    write_stream: &Mutex<TW>,
    in_packets: &mut [RtmpPacketWrapper; IN_PACKETS_BUFFER_SIZE],
) -> bool {
    // Check if the session was killed before reading any chunk

    if session_context.is_killed().await {
        log_debug!(logger, "Session killed");

        return false;
    }

    let mut bytes_read_count: usize = 0; // Counter of read bytes

    // Read start byte

    let start_byte = match tokio::time::timeout(
        Duration::from_secs(RTMP_PING_TIMEOUT),
        read_stream.read_u8(),
    )
    .await
    {
        Ok(br) => match br {
            Ok(b) => b,
            Err(e) => {
                log_debug!(
                    logger,
                    format!("Chunk read error. Could not read start byte: {}", e)
                );

                return false;
            }
        },
        Err(_) => {
            log_debug!(
                logger,
                "Chunk read error. Could not read start byte: Timed out"
            );

            return false;
        }
    };

    bytes_read_count += 1;

    // Read header

    let basic_bytes: usize = if start_byte & 0x3f == 0 {
        2
    } else if start_byte & 0x3f == 1 {
        3
    } else {
        1
    };

    let header_res_bytes_size = get_rtmp_header_size(start_byte >> 6);

    let mut header: Vec<u8> = vec![0; basic_bytes + header_res_bytes_size];

    header[0] = start_byte;

    if basic_bytes > 1 {
        for (i, header_byte) in header.iter_mut().enumerate().take(basic_bytes).skip(1) {
            let basic_byte = match tokio::time::timeout(
                Duration::from_secs(RTMP_PING_TIMEOUT),
                read_stream.read_u8(),
            )
            .await
            {
                Ok(br) => match br {
                    Ok(b) => b,
                    Err(e) => {
                        log_debug!(
                            logger,
                            format!("Chunk read error. Could not read basic byte [{}]: {}", i, e,)
                        );

                        return false;
                    }
                },
                Err(_) => {
                    log_debug!(
                        logger,
                        format!(
                            "Chunk read error. Could not read basic byte [{}]: Timed out",
                            i
                        )
                    );

                    return false;
                }
            };

            *header_byte = basic_byte;

            bytes_read_count += 1;
        }
    }

    if header_res_bytes_size > 0 {
        // Read the rest of the header
        match tokio::time::timeout(
            Duration::from_secs(RTMP_PING_TIMEOUT),
            read_stream.read_exact(&mut header[basic_bytes..]),
        )
        .await
        {
            Ok(r) => {
                if let Err(e) = r {
                    log_debug!(
                        logger,
                        format!("Chunk read error. Could not read header: {}", e)
                    );

                    return false;
                }
            }
            Err(_) => {
                log_debug!(logger, "Chunk read error. Could not read header: Timed out");

                return false;
            }
        };

        bytes_read_count += header_res_bytes_size;
    }

    // Parse packet metadata

    let format = (header[0] >> 6) as u32;

    let channel_id = match basic_bytes {
        2 => 64 + (header[1] as u32),
        3 => (64 + (header[1] as u32) + (header[2] as u32)) << 8,
        _ => (header[0] & 0x3f) as u32,
    };

    // Find the packet in the buffer

    let (packet_buf_index, packet_buf_dropped) =
        get_input_packet_from_buffer(in_packets, channel_id);

    let packet_wrapper = in_packets.get_mut(packet_buf_index).unwrap();

    if packet_buf_dropped {
        log_debug!(
            logger,
            format!(
                "Reusing a packet slot from the buffer: {}",
                packet_buf_index
            )
        );
    }

    packet_wrapper.packet.header.channel_id = channel_id;
    packet_wrapper.packet.header.format = format;

    let mut offset: usize = basic_bytes;

    // Timestamp / delta
    if packet_wrapper.packet.header.format <= RTMP_CHUNK_TYPE_2 {
        if header.len() < offset + 3 {
            if server_context.config.log_requests {
                log_error!(
                    logger,
                    "Header parsing error: Could not parse timestamp/delta"
                );
            }
            return false;
        }

        let ts_bytes = &header[offset..offset + 3];

        packet_wrapper.packet.header.timestamp = ((ts_bytes[2] as u32)
            | ((ts_bytes[1] as u32) << 8)
            | ((ts_bytes[0] as u32) << 16)) as i64;

        offset += 3;
    }

    // Message length + type
    if packet_wrapper.packet.header.format <= RTMP_CHUNK_TYPE_1 {
        if header.len() < offset + 4 {
            if server_context.config.log_requests {
                log_error!(
                    logger,
                    "Header parsing error: Could not parse message length + type"
                );
            }
            return false;
        }

        let ts_bytes = &header[offset..offset + 3];

        packet_wrapper.packet.header.length = ((ts_bytes[2] as u32)
            | ((ts_bytes[1] as u32) << 8)
            | ((ts_bytes[0] as u32) << 16)) as usize;
        packet_wrapper.packet.header.packet_type = header[offset + 3] as u32;

        offset += 4;
    }

    // Stream id
    if packet_wrapper.packet.header.format == RTMP_CHUNK_TYPE_0 {
        if header.len() < offset + 4 {
            if server_context.config.log_requests {
                log_error!(logger, "Header parsing error: Could not parse stream id");
            }
            return false;
        }

        packet_wrapper.packet.header.stream_id =
            LittleEndian::read_u32(&header[offset..offset + 4]);
    }

    // Stop packet
    if packet_wrapper.packet.header.packet_type > RTMP_TYPE_METADATA {
        log_debug!(
            logger,
            format!(
                "Received stop packet: {}",
                packet_wrapper.packet.header.packet_type
            )
        );

        return false;
    }

    // Extended timestamp
    let extended_timestamp: i64 = if packet_wrapper.packet.header.timestamp == 0xffffff {
        let mut ts_bytes: Vec<u8> = vec![0; 4];

        // Read extended timestamp
        match tokio::time::timeout(
            Duration::from_secs(RTMP_PING_TIMEOUT),
            read_stream.read_exact(&mut ts_bytes),
        )
        .await
        {
            Ok(r) => {
                if let Err(e) = r {
                    log_debug!(
                        logger,
                        format!("Chunk read error. Could not read extended timestamp: {}", e)
                    );

                    return false;
                }
            }
            Err(_) => {
                log_debug!(
                    logger,
                    "Chunk read error. Could not read extended timestamp: Timed out"
                );

                return false;
            }
        };

        bytes_read_count += 4;

        BigEndian::read_u32(&ts_bytes) as i64
    } else {
        packet_wrapper.packet.header.timestamp
    };

    if packet_wrapper.bytes == 0 {
        if packet_wrapper.packet.header.format == RTMP_CHUNK_TYPE_0 {
            packet_wrapper.clock = extended_timestamp;
        } else {
            packet_wrapper.clock = packet_wrapper.clock.wrapping_add(extended_timestamp);
        }

        session_context.set_clock(packet_wrapper.clock).await;
    }

    // Packet payload

    let size_to_read: usize = cmp::min(
        session_context.read_status.in_chunk_size
            - (packet_wrapper.bytes % session_context.read_status.in_chunk_size),
        packet_wrapper.packet.header.length - packet_wrapper.bytes,
    );

    if size_to_read > 0 {
        let new_payload_size = packet_wrapper.bytes + size_to_read;
        packet_wrapper
            .packet
            .payload
            .resize(packet_wrapper.bytes + size_to_read, 0);

        // Read payload bytes
        match tokio::time::timeout(
            Duration::from_secs(RTMP_PING_TIMEOUT),
            read_stream.read_exact(
                &mut packet_wrapper.packet.payload[packet_wrapper.bytes..new_payload_size],
            ),
        )
        .await
        {
            Ok(r) => {
                if let Err(e) = r {
                    log_debug!(
                        logger,
                        format!("Chunk read error. Could not read payload bytes: {}", e)
                    );

                    return false;
                }
            }
            Err(_) => {
                log_debug!(
                    logger,
                    "Chunk read error. Could not read payload bytes: Timed out"
                );

                return false;
            }
        };

        bytes_read_count += size_to_read;
        packet_wrapper.bytes = new_payload_size;
    }

    // If packet is ready, handle
    if packet_wrapper.bytes >= packet_wrapper.packet.header.length {
        packet_wrapper.handled = true;

        if packet_wrapper.clock <= 0xffffffff
            && !handle_rtmp_packet(
                logger,
                server_context,
                session_context,
                write_stream,
                &packet_wrapper.packet,
            )
            .await
        {
            log_debug!(logger, "Packet handing failed");

            return false;
        }
    }

    // ACK

    session_context.read_status.in_ack_size = session_context
        .read_status
        .in_ack_size
        .wrapping_add(bytes_read_count);

    if session_context.read_status.in_ack_size >= 0xf0000000 {
        session_context.read_status.in_ack_size = 0;
        session_context.read_status.in_last_ack = 0;
    }

    if session_context.read_status.ack_size > 0
        && session_context.read_status.in_ack_size - session_context.read_status.in_last_ack
            >= session_context.read_status.ack_size
    {
        session_context.read_status.in_last_ack = session_context.read_status.in_ack_size;

        // Send ACK
        let ack_msg = rtmp_make_ack(session_context.read_status.in_ack_size);

        if let Err(e) = session_write_bytes(write_stream, &ack_msg).await {
            log_debug!(logger, format!("Could not send ACK: {}", e));

            return false;
        }

        log_debug!(
            logger,
            format!("Sent ACK: {}", session_context.read_status.in_ack_size)
        );
    }

    // Bitrate

    if server_context.config.log_requests && logger.config.debug_enabled {
        let now = Utc::now().timestamp_millis();
        session_context.read_status.bit_rate_bytes = session_context
            .read_status
            .bit_rate_bytes
            .wrapping_add(bytes_read_count);

        let time_diff = now - session_context.read_status.bit_rate_last_update;

        if time_diff >= BIT_RATE_COMPUTE_INTERVAL_MS {
            let bit_rate = f64::round(
                (session_context.read_status.bit_rate_bytes as f64) * 8.0
                    / ((time_diff as f64) / 1000.0),
            );

            session_context.read_status.bit_rate_bytes = 0;
            session_context.read_status.bit_rate_last_update = now;

            log_debug!(logger, format!("Input bit rate is now: {} bps", bit_rate));
        }
    }

    true
}

/// Gets an input packet from the buffer
///
/// # Arguments
///
/// * `in_packets` - Input packets buffer
/// * `channel_id` - Channel ID
///
/// # Return value
///
/// Returns the index of the slot to use, and true if there were no slots, so the first one was chosen
pub fn get_input_packet_from_buffer(
    in_packets: &mut [RtmpPacketWrapper; IN_PACKETS_BUFFER_SIZE],
    channel_id: u32,
) -> (usize, bool) {
    for (i, item) in in_packets
        .iter_mut()
        .enumerate()
        .take(IN_PACKETS_BUFFER_SIZE)
    {
        if item.packet.header.channel_id == channel_id {
            if item.handled {
                item.reset();
            }
            return (i, false);
        }
    }

    for (i, item) in in_packets
        .iter_mut()
        .enumerate()
        .take(IN_PACKETS_BUFFER_SIZE)
    {
        if !item.used {
            item.used = true;
            return (i, false);
        }
    }

    for (i, item) in in_packets
        .iter_mut()
        .enumerate()
        .take(IN_PACKETS_BUFFER_SIZE)
    {
        if item.handled {
            item.reset_full();
            item.used = true;
            return (i, true);
        }
    }

    in_packets[0].reset_full();

    (0, true)
}
