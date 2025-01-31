// Pause command

use tokio::sync::Mutex;

use crate::{
    log::Logger,
    rtmp::RtmpCommand,
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::super::RtmpSessionStatus;

/// Handles RTMP command (play)
/// cmd - The command to handle
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_pause(
    cmd: &RtmpCommand,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    if !RtmpSessionStatus::check_is_player(session_status).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Pause command ignored since it was not playing");
        }

        return true;
    }

    let channel = match RtmpSessionStatus::get_channel(session_status).await {
        Some(c) => c,
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received pause before connect");
            }

            return false;
        }
    };

    let is_pause = match cmd.get_argument("pause") {
        Some(p) => p.get_bool(),
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Pause command is missing the pause argument");
            }

            return true;
        }
    };

    if is_pause {
        RtmpServerStatus::player_pause(server_status, &channel, session_id).await;
    } else {
        RtmpServerStatus::player_resume(server_status, &channel, session_id).await;
    }

    true
}
