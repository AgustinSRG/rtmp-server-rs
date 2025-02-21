// Receive options commands

use crate::{
    log::Logger,
    rtmp::RtmpCommand,
    server::{player_set_receive_audio, player_set_receive_video, RtmpServerContext},
    session::SessionReadThreadContext,
};

/// Handles RTMP command: RECEIVE AUDIO
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
pub async fn handle_rtmp_command_receive_audio(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    cmd: &RtmpCommand,
) -> bool {
    let receive_audio = match cmd.get_argument("bool") {
        Some(v) => v.get_bool(),
        None => false,
    };

    if server_context.config.log_requests && logger.config.debug_enabled {
        logger.log_debug(&format!("Receive audio setting change: {}", receive_audio));
    }

    let mut session_status_v = session_context.status.lock().await;
    session_status_v.play_status.receive_audio = receive_audio;

    let channel_opt = session_status_v.channel.clone();

    drop(session_status_v);

    if let Some(channel) = channel_opt {
        player_set_receive_audio(server_context, &channel, session_context.id, receive_audio)
            .await;
    }

    true
}

/// Handles RTMP command: RECEIVE VIDEO
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
pub async fn handle_rtmp_command_receive_video(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    cmd: &RtmpCommand,
) -> bool {
    let receive_video = match cmd.get_argument("bool") {
        Some(v) => v.get_bool(),
        None => false,
    };

    if server_context.config.log_requests && logger.config.debug_enabled {
        logger.log_debug(&format!("Receive video setting change: {}", receive_video));
    }

    let mut session_status_v = session_context.status.lock().await;
    session_status_v.play_status.receive_video = receive_video;

    let channel_opt = session_status_v.channel.clone();

    drop(session_status_v);

    if let Some(channel) = channel_opt {
        player_set_receive_video(server_context, &channel, session_context.id, receive_video)
            .await;
    }

    true
}
