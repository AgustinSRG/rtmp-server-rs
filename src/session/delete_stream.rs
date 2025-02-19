// Stream deletion util

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    server::{RtmpServerContext, RtmpServerStatus},
};

use super::{send_status_message, SessionReadThreadContext};

/// Deletes RTMP stream
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `stream_id` - ID of the RTMP stream to delete
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn rtmp_delete_stream<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    stream_id: u32,
) -> bool {
    let mut session_status_v = session_context.status.lock().await;

    let channel = match &session_status_v.channel {
        Some(c) => c.clone(),
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Trying to delete a stream before connect");
            }

            return true;
        }
    };

    let key = match &session_status_v.key {
        Some(k) => k.clone(),
        None => "".to_string(),
    };

    let can_clear_player = session_status_v.is_player;
    let can_clear_publisher = session_status_v.is_publisher;

    let is_play_stream = stream_id == session_status_v.play_stream_id;

    if is_play_stream {
        session_status_v.play_stream_id = 0;
    }

    let is_publish_stream = stream_id == session_status_v.publish_stream_id;

    if is_publish_stream {
        session_status_v.publish_stream_id = 0;
    }

    drop(session_status_v);

    if is_play_stream {
        if server_context.config.log_requests {
            logger.log_info("PLAY STOP");
        }

        if let Err(e) = send_status_message(
            write_stream,
            stream_id,
            "status",
            "NetStream.Play.Stop",
            Some("Stopped playing stream."),
            server_context.config.chunk_size,
        )
        .await
        {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Send error: Could not send status message: {}", e));
            }
        }

        if can_clear_player {
            RtmpServerStatus::remove_player(&server_context.status, &channel, session_context.id)
                .await;
            RtmpServerStatus::try_clear_channel(&server_context.status, &channel).await;
        }
    }

    if is_publish_stream {
        if server_context.config.log_requests {
            logger.log_info("PUBLISH END");
        }

        if let Err(e) = send_status_message(
            write_stream,
            stream_id,
            "status",
            "NetStream.Unpublish.Success",
            Some(&format!("/{}/{} is now unpublished.", channel, key)),
            server_context.config.chunk_size,
        )
        .await
        {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Send error: Could not send status message: {}", e));
            }
        }

        if can_clear_publisher {
            RtmpServerStatus::remove_publisher(
                logger,
                &server_context.config,
                &server_context.status,
                &mut server_context.control_key_validator_sender,
                &channel,
                session_context.id,
            )
            .await;
            RtmpServerStatus::try_clear_channel(&server_context.status, &channel).await;
        }
    }

    true
}
