// Receive options commands

use tokio::sync::Mutex;

use crate::{
    log::Logger,
    rtmp::RtmpCommand,
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::super::RtmpSessionStatus;

/// Handles RTMP command (receiveAudio)
/// cmd - The command to handle
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_receive_audio(
    cmd: &RtmpCommand,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    let receive_audio = match cmd.get_argument("bool") {
        Some(v) => v.get_bool(),
        None => false,
    };

    if config.log_requests && logger.config.debug_enabled {
        logger.log_debug(&format!("Receive audio setting change: {}", receive_audio));
    }

    let mut session_status_v = session_status.lock().await;
    session_status_v.receive_audio = receive_audio;

    let channel_opt = session_status_v.channel.clone();

    drop(session_status_v);

    if let Some(channel) = channel_opt {
        RtmpServerStatus::player_set_receive_audio(
            server_status,
            &channel,
            session_id,
            receive_audio,
        )
        .await;
    }

    true
}

/// Handles RTMP command (receiveVideo)
/// cmd - The command to handle
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_receive_video(
    cmd: &RtmpCommand,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    let receive_video = match cmd.get_argument("bool") {
        Some(v) => v.get_bool(),
        None => false,
    };

    if config.log_requests && logger.config.debug_enabled {
        logger.log_debug(&format!("Receive video setting change: {}", receive_video));
    }

    let mut session_status_v = session_status.lock().await;
    session_status_v.receive_video = receive_video;

    let channel_opt = session_status_v.channel.clone();

    drop(session_status_v);

    if let Some(channel) = channel_opt {
        RtmpServerStatus::player_set_receive_video(
            server_status,
            &channel,
            session_id,
            receive_video,
        )
        .await;
    }

    true
}
