// Play command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    rtmp::{RtmpCommand, RtmpPacket},
    server::{add_player, AddPlayerOptions, RtmpServerContext},
    session::{send_status_message, SessionReadThreadContext},
    utils::{parse_query_string_simple, validate_id_string},
};

use super::super::RtmpSessionStatus;

/// Handles RTMP command: PLAY
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `packet` - The packet that contained the command
/// * `cmd` - The command
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_play<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    packet: &RtmpPacket,
    cmd: &RtmpCommand,
) -> bool {
    // Load and validate parameters

    let play_stream_id = packet.header.stream_id;

    let channel = match RtmpSessionStatus::get_channel(&session_context.status).await {
        Some(c) => c,
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received play before connect");
            }

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "error",
                "NetStream.Play.BadConnection",
                Some("No channel is selected"),
                server_context.config.chunk_size,
            )
            .await
            {
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
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
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: streamName property not provided");
            }

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "error",
                "NetStream.Play.BadName",
                Some("No stream key provided"),
                server_context.config.chunk_size,
            )
            .await
            {
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            return false;
        }
    };

    if !validate_id_string(key, server_context.config.id_max_length) {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!("Command error: Invalid streamName value: {}", key));
        }

        if let Err(e) = send_status_message(
            write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadName",
            Some("Invalid stream key provided"),
            server_context.config.chunk_size,
        )
        .await
        {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Send error: Could not send status message: {}", e));
            }
        }

        return false;
    }

    // Ensure it is not playing

    if RtmpSessionStatus::check_is_player(&session_context.status).await {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Received play command, but already playing");
        }

        if let Err(e) = send_status_message(
            write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadConnection",
            Some("Connection already playing"),
            server_context.config.chunk_size,
        )
        .await
        {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Send error: Could not send status message: {}", e));
            }
        }

        return false;
    }

    // Ensure the client IP is whitelisted

    if !server_context
        .config
        .play_whitelist
        .contains_ip(&session_context.ip)
    {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Attempted to play, but not whitelisted");
        }

        if let Err(e) = send_status_message(
            write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadName",
            Some("Your net address is not whitelisted for playing"),
            server_context.config.chunk_size,
        )
        .await
        {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Send error: Could not send status message: {}", e));
            }
        }

        return false;
    }

    // Log

    if server_context.config.log_requests {
        logger.log_info(&format!("PLAY ({}): {}", play_stream_id, &channel));
    }

    // Update session status

    let (receive_audio, receive_video) =
        RtmpSessionStatus::set_player(&session_context.status, gop_receive, play_stream_id).await;

    // Update server status

    if !add_player(
        server_context,
        session_context,
        &channel,
        key,
        AddPlayerOptions {
            gop_clear,
            receive_audio,
            receive_video,
        },
    )
    .await
    {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Invalid streaming key provided");
        }

        if let Err(e) = send_status_message(
            write_stream,
            play_stream_id,
            "error",
            "NetStream.Play.BadName",
            Some("Invalid stream key provided"),
            server_context.config.chunk_size,
        )
        .await
        {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Send error: Could not send status message: {}", e));
            }
        }

        return false;
    }

    // Done

    true
}
