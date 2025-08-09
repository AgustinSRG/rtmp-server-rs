// Pause command

use crate::{
    log::Logger,
    log_debug,
    rtmp::RtmpCommand,
    server::{player_pause, player_resume, RtmpServerContext},
    session::SessionReadThreadContext,
};

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
    if !session_context.is_player().await {
        log_debug!(logger, "Pause command ignored since it was not playing");

        return true;
    }

    let channel = match session_context.channel().await {
        Some(c) => c,
        None => {
            log_debug!(logger, "Protocol error: Received pause before connect");

            return false;
        }
    };

    let is_pause = match cmd.get_argument("pause") {
        Some(p) => p.get_bool(),
        None => {
            log_debug!(logger, "Pause command is missing the pause argument");

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
