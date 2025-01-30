// Play command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{RtmpCommand, RtmpPacket},
    server::{RtmpServerConfiguration, RtmpServerStatus},
    session::send_status_message,
    utils::{parse_query_string_simple, validate_id_string},
};

use super::super::{RtmpSessionMessage, RtmpSessionReadStatus, RtmpSessionStatus};

/// Handles RTMP command (play)
/// packet - The packet to handle
/// cmd - The command to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// session_msg_sender - Message sender for the session
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_play<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    packet: &RtmpPacket,
    cmd: &RtmpCommand,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    read_status: &mut RtmpSessionReadStatus,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let play_stream_id = packet.header.stream_id;

    let channel = match RtmpSessionStatus::get_channel(session_status).await {
        Some(c) => c,
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received publish before connect");
            }

            if let Err(e) = send_status_message(
                &write_stream,
                play_stream_id,
                "error",
                "NetStream.Play.BadConnection",
                Some("No channel is selected"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Send error: Could not send status message: {}",
                        e.to_string()
                    ));
                }
            }

            return false;
        }
    };

    let (key, gop_receive, gop_clear) = match cmd.get_argument("streamName") {
        Some(k) => {
            let k_parts: Vec<&str> = k.get_string().split("?").collect();

            if k_parts.len() > 1 {
                let q_str = parse_query_string_simple(k_parts[1]);

                match q_str.get("cache") {
                    Some(cache_opt) => match cache_opt.as_str() {
                        "clear" => (k_parts[0], true, false),
                        "no" => (k_parts[0], false, false),
                        _ => (k_parts[0], true, false),
                    },
                    None => (k_parts[0], true, false),
                }
            } else {
                (k.get_string(), true, false)
            }
        }
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: streamName property not provided");
            }

            if let Err(e) = send_status_message(
                &write_stream,
                play_stream_id,
                "error",
                "NetStream.Play.BadName",
                Some("No stream key provided"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Send error: Could not send status message: {}",
                        e.to_string()
                    ));
                }
            }

            return false;
        }
    };

    if !validate_id_string(key, config.id_max_length) {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!("Command error: Invalid streamName value: {}", key));
        }

        if let Err(e) = send_status_message(
            &write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadName",
            Some("Invalid stream key provided"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Ensure it is not playing

    if !RtmpSessionStatus::check_is_player(session_status).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Received play command, but already playing");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadConnection",
            Some("Connection already playing"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Ensure the client IP is whitelisted

    if !config.play_whitelist.contains_ip(&read_status.ip) {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Attempted to play, but not whitelisted");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadName",
            Some("Your net address is not whitelisted for playing"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Log

    if config.log_requests {
        logger.log_info(&format!("PLAY ({}): {}", play_stream_id, &channel));
    }

    // Update session status

    let (receive_audio, receive_video) =
        RtmpSessionStatus::set_player(session_status, gop_receive, play_stream_id).await;

    // Update server status

    if !RtmpServerStatus::add_player(
        server_status,
        &channel,
        key,
        session_id,
        session_msg_sender.clone(),
        gop_clear,
        receive_audio,
        receive_video,
    )
    .await
    {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Invalid streaming key provided");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadName",
            Some("Invalid stream key provided"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Done

    true
}
