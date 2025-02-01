// Chunk read logic

use std::{cmp, collections::HashMap, sync::Arc, time::Duration};

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use chrono::Utc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    control::ControlKeyValidationRequest, log::Logger, rtmp::{
        get_rtmp_header_size, rtmp_make_ack, RtmpPacket, RTMP_CHUNK_TYPE_0, RTMP_CHUNK_TYPE_1,
        RTMP_CHUNK_TYPE_2, RTMP_PING_TIMEOUT, RTMP_TYPE_METADATA,
    }, server::{RtmpServerConfiguration, RtmpServerStatus}
};

use super::{
    handle_rtmp_packet, session_write_bytes, RtmpSessionMessage, RtmpSessionPublishStreamStatus,
    RtmpSessionReadStatus, RtmpSessionStatus,
};

/// Interval to compute bit rate (milliseconds)
const BIT_RATE_COMPUTE_INTERVAL_MS: i64 = 1000;

/// Reads RTMP chunk and, if ready, handles it
/// session_id - Session ID
/// read_stream - IO stream to read bytes
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// session_msg_sender - Message sender for the session
/// session_msg_receiver - Message receiver for the session
/// read_status_mu - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunk. Returns false to end the session main loop.
pub async fn read_rtmp_chunk<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    session_id: u64,
    read_stream: &mut TR,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    read_status: &mut RtmpSessionReadStatus,
    in_packets: &mut HashMap<u32, RtmpPacket>,
    control_key_validator_sender: &mut Option<Sender<ControlKeyValidationRequest>>,
    logger: &Logger,
) -> bool {
    // Check if the session was killed before reading any chunk

    if RtmpSessionStatus::is_killed(session_status).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Session killed");
        }
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
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Chunk read error. Could not read start byte: {}",
                        e.to_string()
                    ));
                }
                return false;
            }
        },
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Chunk read error. Could not read start byte: Timed out");
            }
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

    let mut header: Vec<u8> = vec![0; 1 + basic_bytes + header_res_bytes_size];

    header[0] = start_byte;

    for i in 0..basic_bytes {
        let basic_byte = match tokio::time::timeout(
            Duration::from_secs(RTMP_PING_TIMEOUT),
            read_stream.read_u8(),
        )
        .await
        {
            Ok(br) => match br {
                Ok(b) => b,
                Err(e) => {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Chunk read error. Could not read basic byte [{}]: {}",
                            i,
                            e.to_string(),
                        ));
                    }
                    return false;
                }
            },
            Err(_) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Chunk read error. Could not read basic byte [{}]: Timed out",
                        i
                    ));
                }
                return false;
            }
        };

        header[i + 1] = basic_byte;

        bytes_read_count += 1;
    }

    if header_res_bytes_size > 0 {
        // Read the rest of the header
        match tokio::time::timeout(
            Duration::from_secs(RTMP_PING_TIMEOUT),
            read_stream.read_exact(&mut header[1 + basic_bytes..]),
        )
        .await
        {
            Ok(r) => {
                if let Err(e) = r {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Chunk read error. Could not read header: {}",
                            e.to_string()
                        ));
                    }
                    return false;
                }
            }
            Err(_) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Chunk read error. Could not read header: Timed out");
                }
                return false;
            }
        };

        bytes_read_count += header_res_bytes_size;
    }

    // Parse packet metadata

    let format = (header[0] >> 6) as u32;

    let channel_id = match basic_bytes {
        2 => 64 + (header[1] as u32),
        3 => (64 + (header[1] as u32)) + (header[2] as u32) << 8,
        _ => (header[0] & 0x3f) as u32,
    };

    let packet = match in_packets.get_mut(&channel_id) {
        Some(p) => {
            if p.handled {
                p.handled = false;
                p.payload = Vec::new();
                p.bytes = 0;
            }
            p
        }
        None => {
            let p = RtmpPacket::new_blank();

            in_packets.insert(channel_id, p);

            match in_packets.get_mut(&channel_id) {
                Some(p) => p,
                None => {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug("Could not create RTMP packet.");
                    }
                    return false;
                }
            }
        }
    };

    packet.header.channel_id = channel_id;
    packet.header.format = format;

    let mut offset: usize = basic_bytes;

    // Timestamp / delta
    if packet.header.format <= RTMP_CHUNK_TYPE_2 {
        if header.len() < offset + 3 {
            if config.log_requests {
                logger.log_error("Header parsing error: Could not parse timestamp/delta");
            }
            return false;
        }

        let ts_bytes = &header[offset..offset + 3];

        packet.header.timestamp = ((ts_bytes[2] as u32)
            | ((ts_bytes[1] as u32) << 8)
            | ((ts_bytes[0] as u32) << 16)) as i64;

        offset += 3;
    }

    // Message length + type
    if packet.header.format <= RTMP_CHUNK_TYPE_1 {
        if header.len() < offset + 4 {
            if config.log_requests {
                logger.log_error("Header parsing error: Could not parse message length + type");
            }
            return false;
        }

        let ts_bytes = &header[offset..offset + 3];

        packet.header.length = ((ts_bytes[2] as u32)
            | ((ts_bytes[1] as u32) << 8)
            | ((ts_bytes[0] as u32) << 16)) as usize;
        packet.header.packet_type = header[offset + 3] as u32;

        offset += 4;
    }

    // Stream id
    if packet.header.format == RTMP_CHUNK_TYPE_0 {
        if header.len() < offset + 4 {
            if config.log_requests {
                logger.log_error("Header parsing error: Could not parse stream id");
            }
            return false;
        }

        packet.header.stream_id = LittleEndian::read_u32(&header[offset..offset + 4]);
    }

    // Stop packet
    if packet.header.packet_type > RTMP_TYPE_METADATA {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Received stop packet: {}",
                packet.header.packet_type
            ));
        }
        return false;
    }

    // Extended timestamp
    let extended_timestamp: i64 = if packet.header.timestamp == 0xffffff {
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
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Chunk read error. Could not read extended timestamp: {}",
                            e.to_string()
                        ));
                    }
                    return false;
                }
            }
            Err(_) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(
                        "Chunk read error. Could not read extended timestamp: Timed out",
                    );
                }
                return false;
            }
        };

        bytes_read_count += 4;

        BigEndian::read_u32(&ts_bytes) as i64
    } else {
        packet.header.timestamp
    };

    if packet.bytes == 0 {
        if packet.header.format == RTMP_CHUNK_TYPE_0 {
            packet.clock = extended_timestamp;
        } else {
            packet.clock = packet.clock.wrapping_add(extended_timestamp);
        }

        RtmpSessionPublishStreamStatus::set_clock(&publish_status, packet.clock).await;

        if packet.capacity < packet.header.length {
            packet.capacity = packet.header.length.wrapping_add(1024);
        }
    }

    // Packet payload

    let size_to_read: usize = cmp::max(
        read_status.in_chunk_size - (packet.bytes % read_status.in_chunk_size),
        packet.header.length - packet.bytes,
    );

    if size_to_read > 0 {
        let mut payload_bytes: Vec<u8> = vec![0; size_to_read];

        // Read payload bytes
        match tokio::time::timeout(
            Duration::from_secs(RTMP_PING_TIMEOUT),
            read_stream.read_exact(&mut payload_bytes),
        )
        .await
        {
            Ok(r) => {
                if let Err(e) = r {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Chunk read error. Could not read payload bytes: {}",
                            e.to_string()
                        ));
                    }
                    return false;
                }
            }
            Err(_) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Chunk read error. Could not read payload bytes: Timed out");
                }
                return false;
            }
        };

        bytes_read_count += size_to_read;

        packet.bytes += size_to_read;
        packet.payload.extend(payload_bytes);
    }

    // If packet is ready, handle
    if packet.bytes >= packet.header.length {
        packet.handled = true;

        if packet.clock <= 0xffffffff {
            if !handle_rtmp_packet(
                packet,
                session_id,
                write_stream,
                config,
                server_status,
                session_status,
                publish_status,
                session_msg_sender,
                read_status,
                control_key_validator_sender,
                logger,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Packet handing failed");
                }
                return false;
            }
        }
    }

    // ACK

    read_status.in_ack_size = read_status.in_ack_size.wrapping_add(bytes_read_count);

    if read_status.in_ack_size >= 0xf0000000 {
        read_status.in_ack_size = 0;
        read_status.in_last_ack = 0;
    }

    if read_status.ack_size > 0
        && read_status.in_ack_size - read_status.in_last_ack >= read_status.ack_size
    {
        read_status.in_last_ack = read_status.in_ack_size;

        // Send ACK
        let ack_msg = rtmp_make_ack(read_status.in_ack_size);

        if let Err(e) = session_write_bytes(write_stream, &ack_msg).await {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Could not send ACK: {}", e.to_string()));
            }
            return false;
        }

        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!("Sent ACK: {}", read_status.in_ack_size));
        }
    }

    // Bitrate

    if config.log_requests && logger.config.trace_enabled {
        let now = Utc::now().timestamp_millis();
        read_status.bit_rate_bytes = read_status.bit_rate_bytes.wrapping_add(bytes_read_count);

        let time_diff = now - read_status.bit_rate_last_update;

        if time_diff >= BIT_RATE_COMPUTE_INTERVAL_MS {
            let bit_rate = f64::round(
                (read_status.bit_rate_bytes as f64) * 8.0 / ((time_diff as f64) / 1000.0),
            );

            read_status.bit_rate_bytes = 0;
            read_status.bit_rate_last_update = now;

            logger.log_trace(&format!("Bitrate is now: {} bps", bit_rate));
        }
    }

    return true;
}
