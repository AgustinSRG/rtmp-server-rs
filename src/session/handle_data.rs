// Logic to handle data packets

use std::sync::Arc;

use crate::{
    log::Logger,
    log_debug, log_error, log_trace,
    rtmp::{rtmp_build_metadata, RtmpData, RtmpPacket, RTMP_TYPE_FLEX_STREAM},
    server::{set_channel_metadata, RtmpServerContext},
};

use super::SessionReadThreadContext;

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
        log_debug!(logger, "Packet error: Packet length too short");

        return false;
    }

    if packet.header.length > packet.payload.len() {
        log_error!(
            logger,
            "Packet error: Payload does not match with packet length"
        );

        return false;
    }

    let data = match RtmpData::decode(&packet.payload[offset..packet.header.length]) {
        Ok(c) => c,
        Err(_) => {
            log_debug!(logger, "Packet error: Could not decode RTMP data");

            return false;
        }
    };

    log_trace!(logger, format!("DATA RECEIVED: {}", data.to_debug_string()));

    match data.tag.as_str() {
        "@setDataFrame" => {
            let metadata = Arc::new(rtmp_build_metadata(&data));
            let metadata_size = metadata.len();

            let channel_opt = session_context.channel().await;

            if let Some(channel) = channel_opt {
                set_channel_metadata(server_context, &channel, session_context.id, metadata).await;

                log_debug!(
                    logger,
                    format!(
                        "Set channel metadata: {} -> {} bytes",
                        channel, metadata_size
                    )
                );
            }

            true
        }
        _ => {
            log_debug!(logger, format!("Unrecognized data: {}", data.tag));

            true
        }
    }
}
