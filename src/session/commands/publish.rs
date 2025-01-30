// Publish command

use std::sync::Arc;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{RtmpCommand, RtmpPacket},
    server::{RtmpServerConfiguration, RtmpServerStatus},
    utils::validate_id_string,
};

use super::super::{
    send_status_message, RtmpSessionMessage, RtmpSessionPublishStreamStatus, RtmpSessionStatus,
};

/// Handles RTMP command (publish)
/// packet - The packet to handle
/// cmd - The command to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// session_msg_sender - Message sender for the session
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_publish<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    packet: &RtmpPacket,
    cmd: &RtmpCommand,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let publish_stream_id = packet.header.stream_id;

    let channel = match RtmpSessionStatus::get_channel(session_status).await {
        Some(c) => c,
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received publish before connect");
            }

            if let Err(e) = send_status_message(
                &write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadConnection",
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

    let key = match cmd.get_argument("streamName") {
        Some(k) => k.get_string(),
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: streamName property not provided");
            }

            if let Err(e) = send_status_message(
                &write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadName",
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
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
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

    // Ensure the session is not already publishing

    if !RtmpSessionStatus::check_is_publisher(session_status).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Received publish command, but already publishing");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadConnection",
            Some("Connection already publishing"),
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

    // Ensure the channel is free to publish

    if !RtmpServerStatus::check_channel_publishing_status(server_status, &channel).await {
        if config.log_requests && logger.config.debug_enabled {
            logger
                .log_debug("Cannot publish: Another session is already publishing on the channel");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Stream already publishing"),
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
        logger.log_info(&format!("PUBLISH ({}): {}", publish_stream_id, &channel));
    }

    // Check validity of the key (callback or coordinator)

    let stream_id = "";
    logger.log_debug("TODO");

    // Set publisher into the server status

    if !RtmpServerStatus::set_publisher(
        server_status,
        &channel,
        key,
        stream_id,
        session_id,
        publish_status.clone(),
        session_msg_sender.clone(),
    )
    .await
    {
        if config.log_requests && logger.config.debug_enabled {
            logger
                .log_debug("Cannot publish: Another session is already publishing on the channel");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Stream already publishing"),
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

    // Set publishing status to the session status

    RtmpSessionStatus::set_publisher(session_status, true, publish_stream_id).await;

    // Respond with status message

    if let Err(e) = send_status_message(
        &write_stream,
        publish_stream_id,
        "status",
        "NetStream.Publish.Start",
        Some(&format!("/{}/{} is now published.", channel, key)),
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

    // Done

    true
}
