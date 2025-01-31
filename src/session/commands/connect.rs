// Connect command

use chrono::Utc;
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    rtmp::{
        rtmp_make_chunk_size_set_message, rtmp_make_connect_response,
        rtmp_make_peer_bandwidth_set_message, rtmp_make_window_ack, RtmpCommand,
        RTMP_PEER_BANDWIDTH, RTMP_WINDOW_ACK,
    },
    server::RtmpServerConfiguration,
    utils::validate_id_string,
};

use super::super::{session_write_bytes, RtmpSessionStatus};

/// Handles RTMP command (connect)
/// cmd - The command to handle
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_connect<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    cmd: &RtmpCommand,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let channel = match cmd.get_argument("cmdObj") {
        Some(cmd_obj) => match cmd_obj.get_object_property("app") {
            Some(app) => {
                let app_str = app.get_string();

                if !validate_id_string(app_str, config.id_max_length) {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!("Command error: Invalid app value: {}", app_str));
                    }

                    return false;
                }

                app_str
            }
            None => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Command error: app property not provided");
                }

                return false;
            }
        },
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: cmdObj argument not provided");
            }

            return false;
        }
    };

    let object_encoding: Option<u32> = match cmd.get_argument("cmdObj") {
        Some(cmd_obj) => match cmd_obj.get_object_property("objectEncoding") {
            Some(oe) => {
                if !oe.is_undefined() {
                    Some(oe.get_integer() as u32)
                } else {
                    None
                }
            }
            None => None,
        },
        None => None,
    };

    let trans_id = match cmd.get_argument("transId") {
        Some(t) => t.get_integer(),
        None => 0,
    };

    let now = Utc::now().timestamp_millis();

    // Update the session status

    let mut session_status_v = session_status.lock().await;

    if let Some(_) = session_status_v.channel {
        // Already connected. This command is invalid
        drop(session_status_v);
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Connect received, but already connected");
        }

        return false;
    }

    session_status_v.channel = Some(channel.to_string());
    session_status_v.connect_time = now;

    drop(session_status_v);

    // Send window ACK

    let window_ack_bytes = rtmp_make_window_ack(RTMP_WINDOW_ACK);
    if let Err(e) = session_write_bytes(&write_stream, &window_ack_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send window ACK: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Set peer bandwidth

    let peer_bandwidth_bytes = rtmp_make_peer_bandwidth_set_message(RTMP_PEER_BANDWIDTH);
    if let Err(e) = session_write_bytes(&write_stream, &peer_bandwidth_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not set peer bandwidth: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Set chunk size

    let chunk_size_bytes = rtmp_make_chunk_size_set_message(config.chunk_size as u32);
    if let Err(e) = session_write_bytes(&write_stream, &chunk_size_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not set chunk size: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Respond

    let connect_response_bytes =
        rtmp_make_connect_response(trans_id, object_encoding, config.chunk_size);
    if let Err(e) = session_write_bytes(&write_stream, &connect_response_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send connect response: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Done

    true
}
