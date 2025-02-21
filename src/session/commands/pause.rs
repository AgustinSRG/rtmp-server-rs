// Pause command

use crate::{
    log::Logger,
    rtmp::RtmpCommand,
    server::{player_pause, player_resume, RtmpServerContext},
    session::SessionReadThreadContext,
};

use super::super::RtmpSessionStatus;

/// Handles RTMP command: PAUSE
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `cmd` - The command
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_pause(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    cmd: &RtmpCommand,
) -> bool {
    if !RtmpSessionStatus::check_is_player(&session_context.status).await {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Pause command ignored since it was not playing");
        }

        return true;
    }

    let channel = match RtmpSessionStatus::get_channel(&session_context.status).await {
        Some(c) => c,
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received pause before connect");
            }

            return false;
        }
    };

    let is_pause = match cmd.get_argument("pause") {
        Some(p) => p.get_bool(),
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Pause command is missing the pause argument");
            }

            return true;
        }
    };

    if is_pause {
        player_pause(server_context, &channel, session_context.id).await;
    } else {
        player_resume(server_context, &channel, session_context.id).await;
    }

    true
}
