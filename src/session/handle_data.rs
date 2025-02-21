// Logic to handle data packets

use std::sync::Arc;

use crate::{
    log::Logger,
    rtmp::{rtmp_build_metadata, RtmpData, RtmpPacket, RTMP_TYPE_FLEX_STREAM},
    server::{set_channel_metadata, RtmpServerContext},
};

use super::{RtmpSessionStatus, SessionReadThreadContext};

/// Handles DATA RTMP packet
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `packet` - The packet
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet_data(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    packet: &RtmpPacket,
) -> bool {
    let offset: usize = if packet.header.packet_type == RTMP_TYPE_FLEX_STREAM {
        1
    } else {
        0
    };

    if packet.header.length <= offset {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Packet error: Packet length too short");
        }

        return false;
    }

    if packet.header.length > packet.payload.len() {
        if server_context.config.log_requests {
            logger.log_error("Packet error: Payload does not match with packet length");
        }

        return false;
    }

    let data = match RtmpData::decode(&packet.payload[offset..packet.header.length]) {
        Ok(c) => c,
        Err(_) => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Packet error: Could not decode RTMP data");
            }

            return false;
        }
    };

    if server_context.config.log_requests && logger.config.trace_enabled {
        logger.log_trace(&format!("DATA RECEIVED: {}", data.to_debug_string()));
    }

    match data.tag.as_str() {
        "@setDataFrame" => {
            let metadata = Arc::new(rtmp_build_metadata(&data));
            let metadata_size = metadata.len();

            let channel_opt = RtmpSessionStatus::get_channel(&session_context.status).await;

            if let Some(channel) = channel_opt {
                set_channel_metadata(server_context, &channel, session_context.id, metadata).await;

                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Set channel metadata: {} -> {} bytes",
                        channel, metadata_size
                    ));
                }
            }

            true
        }
        _ => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Unrecognized data: {}", data.tag));
            }

            true
        }
    }
}
