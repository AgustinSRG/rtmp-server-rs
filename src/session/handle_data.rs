// Logic to handle data packets

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    log::Logger,
    rtmp::{rtmp_build_metadata, RtmpData, RtmpPacket, RTMP_TYPE_FLEX_STREAM},
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::RtmpSessionStatus;

/// Handles RTMP packet (DATA)
/// packet - The packet to handle
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet_data(
    packet: &RtmpPacket,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    let offset: usize = if packet.header.packet_type == RTMP_TYPE_FLEX_STREAM {
        1
    } else {
        0
    };

    if packet.header.length <= offset {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Packet error: Packet length too short");
        }

        return false;
    }

    if packet.header.length > packet.payload.len() {
        if config.log_requests {
            logger.log_error("Packet error: Payload does not match with packet length");
        }

        return false;
    }

    let data = match RtmpData::decode(&packet.payload[offset..packet.header.length]) {
        Ok(c) => c,
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Packet error: Could not decode RTMP data");
            }

            return false;
        }
    };

    if config.log_requests && logger.config.trace_enabled {
        logger.log_trace(&format!("DATA RECEIVED: {}", data.to_debug_string()));
    }

    match data.tag.as_str() {
        "@setDataFrame" => {
            let metadata = Arc::new(rtmp_build_metadata(&data));
            let metadata_size = metadata.len();

            let channel_opt = RtmpSessionStatus::get_channel(session_status).await;

            if let Some(channel) = channel_opt {
                RtmpServerStatus::set_channel_metadata(
                    server_status,
                    &channel,
                    session_id,
                    metadata,
                )
                .await;

                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Set channel metadata: {} -> {} bytes", channel, metadata_size));
                }
            }

            true
        }
        _ => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Unrecognized data: {}", data.tag));
            }

            true
        }
    }
}
