// Packet handling logic

use std::sync::Arc;

use byteorder::{BigEndian, ByteOrder};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{
        RtmpPacket, RTMP_CHUNK_SIZE, RTMP_MAX_CHUNK_SIZE, RTMP_TYPE_AUDIO, RTMP_TYPE_DATA,
        RTMP_TYPE_FLEX_MESSAGE, RTMP_TYPE_FLEX_STREAM, RTMP_TYPE_INVOKE, RTMP_TYPE_SET_CHUNK_SIZE,
        RTMP_TYPE_VIDEO, RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE,
    },
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{
    handle_rtmp_packet_audio, handle_rtmp_packet_data, handle_rtmp_packet_invoke,
    handle_rtmp_packet_video, RtmpSessionMessage, RtmpSessionPublishStreamStatus,
    RtmpSessionReadStatus, RtmpSessionStatus,
};

/// Handles RTMP packet
/// packet - The packet to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// session_msg_sender - Message sender for the session
/// session_msg_receiver - Message receiver for the session
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    packet: &RtmpPacket,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    read_status: &mut RtmpSessionReadStatus,
    logger: &Logger,
) -> bool {
    match packet.header.packet_type {
        // Packet to set chunk size
        RTMP_TYPE_SET_CHUNK_SIZE => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_SET_CHUNK_SIZE");
            }

            if packet.payload.len() < 4 {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Packet error: Payload too short");
                }

                return false;
            }

            read_status.in_chunk_size = BigEndian::read_u32(&packet.payload[0..4]) as usize;

            if read_status.in_chunk_size < RTMP_CHUNK_SIZE {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Packet error: Chunk size too small. Size: {}. Min: {}",
                        read_status.in_chunk_size, RTMP_CHUNK_SIZE
                    ));
                }

                return false;
            }

            if read_status.in_chunk_size > RTMP_MAX_CHUNK_SIZE {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Packet error: Chunk size too large. Size: {}. Max: {}",
                        read_status.in_chunk_size, RTMP_MAX_CHUNK_SIZE
                    ));
                }

                return false;
            }

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Chunk size updated: {}",
                    read_status.in_chunk_size
                ));
            }

            true
        }
        // Packet to set ACK size
        RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE");
            }

            if packet.payload.len() < 4 {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Packet error: Payload too short");
                }

                return false;
            }

            read_status.ack_size = BigEndian::read_u32(&packet.payload[0..4]) as usize;

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("ACK size updated: {}", read_status.ack_size));
            }

            true
        }
        RTMP_TYPE_AUDIO => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_AUDIO");
            }

            handle_rtmp_packet_audio(
                packet,
                session_id,
                write_stream,
                config,
                server_status,
                session_status,
                publish_status,
                session_msg_sender,
                read_status,
                logger,
            )
            .await
        }
        RTMP_TYPE_VIDEO => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_VIDEO");
            }

            handle_rtmp_packet_video(
                packet,
                session_id,
                write_stream,
                config,
                server_status,
                session_status,
                publish_status,
                session_msg_sender,
                read_status,
                logger,
            )
            .await
        }
        RTMP_TYPE_FLEX_MESSAGE => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_FLEX_MESSAGE");
            }

            handle_rtmp_packet_invoke(
                packet,
                session_id,
                write_stream,
                config,
                server_status,
                session_status,
                publish_status,
                session_msg_sender,
                read_status,
                logger,
            )
            .await
        }
        RTMP_TYPE_INVOKE => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_INVOKE");
            }

            handle_rtmp_packet_invoke(
                packet,
                session_id,
                write_stream,
                config,
                server_status,
                session_status,
                publish_status,
                session_msg_sender,
                read_status,
                logger,
            )
            .await
        }
        RTMP_TYPE_DATA => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_DATA");
            }

            handle_rtmp_packet_data(
                packet,
                session_id,
                config,
                server_status,
                session_status,
                logger,
            )
            .await
        }
        RTMP_TYPE_FLEX_STREAM => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Received packet: RTMP_TYPE_FLEX_STREAM");
            }

            handle_rtmp_packet_data(
                packet,
                session_id,
                config,
                server_status,
                session_status,
                logger,
            )
            .await
        }
        _ => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Received unknown packet type: {}",
                    packet.header.packet_type
                ));
            }

            true
        }
    }
}
