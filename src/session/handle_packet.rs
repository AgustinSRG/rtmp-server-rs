// Packet handling logic

use byteorder::{BigEndian, ByteOrder};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    rtmp::{
        RtmpPacket, RTMP_CHUNK_SIZE, RTMP_MAX_CHUNK_SIZE, RTMP_TYPE_AUDIO, RTMP_TYPE_DATA,
        RTMP_TYPE_FLEX_MESSAGE, RTMP_TYPE_FLEX_STREAM, RTMP_TYPE_INVOKE, RTMP_TYPE_SET_CHUNK_SIZE,
        RTMP_TYPE_VIDEO, RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE,
    },
    server::RtmpServerContext,
};

use super::{
    handle_rtmp_packet_audio, handle_rtmp_packet_data, handle_rtmp_packet_invoke,
    handle_rtmp_packet_video, SessionReadThreadContext,
};

/// Handles parsed RTMP packet
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `packet` - The packet
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    packet: &RtmpPacket,
) -> bool {
    match packet.header.packet_type {
        RTMP_TYPE_SET_CHUNK_SIZE => {
            // Packet to set chunk size
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_SET_CHUNK_SIZE");
            }

            if packet.payload.len() < 4 {
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Packet error: Payload too short");
                }

                return false;
            }

            session_context.read_status.in_chunk_size =
                BigEndian::read_u32(&packet.payload[0..4]) as usize;

            if session_context.read_status.in_chunk_size < RTMP_CHUNK_SIZE {
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Packet error: Chunk size too small. Size: {}. Min: {}",
                        session_context.read_status.in_chunk_size, RTMP_CHUNK_SIZE
                    ));
                }

                return false;
            }

            if session_context.read_status.in_chunk_size > RTMP_MAX_CHUNK_SIZE {
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Packet error: Chunk size too large. Size: {}. Max: {}",
                        session_context.read_status.in_chunk_size, RTMP_MAX_CHUNK_SIZE
                    ));
                }

                return false;
            }

            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Chunk size updated: {}",
                    session_context.read_status.in_chunk_size
                ));
            }

            true
        }
        RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE => {
            // Packet to set ACK size
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE");
            }

            if packet.payload.len() < 4 {
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Packet error: Payload too short");
                }

                return false;
            }

            session_context.read_status.ack_size =
                BigEndian::read_u32(&packet.payload[0..4]) as usize;

            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "ACK size updated: {}",
                    session_context.read_status.ack_size
                ));
            }

            true
        }
        RTMP_TYPE_AUDIO => {
            // Audio packet
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_AUDIO");
            }

            handle_rtmp_packet_audio(logger, server_context, session_context, packet).await
        }
        RTMP_TYPE_VIDEO => {
            // Video packet
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_VIDEO");
            }

            handle_rtmp_packet_video(logger, server_context, session_context, packet).await
        }
        RTMP_TYPE_INVOKE => {
            // Invoke / Command packet
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_INVOKE");
            }

            handle_rtmp_packet_invoke(
                logger,
                server_context,
                session_context,
                write_stream,
                packet,
            )
            .await
        }
        RTMP_TYPE_FLEX_MESSAGE => {
            // Invoke / Command packet (Alt)
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_FLEX_MESSAGE");
            }

            handle_rtmp_packet_invoke(
                logger,
                server_context,
                session_context,
                write_stream,
                packet,
            )
            .await
        }
        RTMP_TYPE_DATA => {
            // Data packet
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_DATA");
            }

            handle_rtmp_packet_data(logger, server_context, session_context, packet).await
        }
        RTMP_TYPE_FLEX_STREAM => {
            // Data packet (Alt)
            if server_context.config.log_requests && logger.config.trace_enabled {
                logger.log_trace("Received packet: RTMP_TYPE_FLEX_STREAM");
            }

            handle_rtmp_packet_data(logger, server_context, session_context, packet).await
        }
        _ => {
            // Other type (not supported by this server implementation)
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Received unknown packet type: {}",
                    packet.header.packet_type
                ));
            }

            true
        }
    }
}
