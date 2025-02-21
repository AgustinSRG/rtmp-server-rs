// Publish command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    callback::make_start_callback,
    control::control_validate_key,
    log::Logger,
    rtmp::{RtmpCommand, RtmpPacket},
    server::{check_channel_publishing_status, set_publisher, RtmpServerContext},
    session::SessionReadThreadContext,
    utils::validate_id_string,
};

use super::super::send_status_message;

/// Handles RTMP command: PUBLISH
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
pub async fn handle_rtmp_command_publish<
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

    let publish_stream_id = packet.header.stream_id;

    let channel = match session_context.channel().await {
        Some(c) => c,
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received publish before connect");
            }

            if let Err(e) = send_status_message(
                write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadConnection",
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

    let key = match cmd.get_argument("streamName") {
        Some(k) => {
            let k_parts: Vec<&str> = k.get_string().split("?").collect();

            if !k_parts.is_empty() {
                k_parts[0]
            } else {
                k.get_string()
            }
        }
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: streamName property not provided");
            }

            if let Err(e) = send_status_message(
                write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadName",
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
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
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

    // Ensure the session is not already publishing

    if session_context.is_publisher().await {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Received publish command, but already publishing");
        }

        if let Err(e) = send_status_message(
            write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadConnection",
            Some("Connection already publishing"),
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

    // Ensure the channel is free to publish

    if check_channel_publishing_status(server_context, &channel).await {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger
                .log_debug("Cannot publish: Another session is already publishing on the channel");
        }

        if let Err(e) = send_status_message(
            write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Stream already publishing"),
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
        logger.log_info(&format!("PUBLISH ({}): {}", publish_stream_id, &channel));
    }

    // Check validity of the key (callback or coordinator)

    let stream_id_res = match &server_context.control_key_validator_sender {
        Some(control_key_validator_sender_v) => {
            control_validate_key(
                control_key_validator_sender_v,
                &channel,
                key,
                &session_context.ip,
            )
            .await
        }
        None => {
            make_start_callback(
                logger,
                &server_context.config.callback,
                &channel,
                key,
                &session_context.ip,
            )
            .await
        }
    };

    let stream_id = match stream_id_res {
        Some(s) => s,
        None => {
            if let Err(e) = send_status_message(
                write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadName",
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
    };

    // Set publisher into the server status

    if !set_publisher(server_context, session_context, &channel, key, &stream_id).await {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger
                .log_debug("Cannot publish: Another session is already publishing on the channel");
        }

        if let Err(e) = send_status_message(
            write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Stream already publishing"),
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

    // Set publishing status to the session status

    session_context.set_publisher(publish_stream_id).await;

    // Respond with status message

    if let Err(e) = send_status_message(
        write_stream,
        publish_stream_id,
        "status",
        "NetStream.Publish.Start",
        Some(&format!("/{}/{} is now published.", channel, key)),
        server_context.config.chunk_size,
    )
    .await
    {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!("Send error: Could not send status message: {}", e));
        }
    }

    // Done

    true
}
