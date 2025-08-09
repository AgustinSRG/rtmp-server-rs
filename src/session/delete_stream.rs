// Stream deletion util

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    log_debug, log_info,
    server::{remove_player, remove_publisher, try_clear_channel, RtmpServerContext},
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
            log_debug!(
                logger,
                "Protocol error: Trying to delete a stream before connect"
            );

            return true;
        }
    };

    let key = match &session_status_v.key {
        Some(k) => k.clone(),
        None => "".to_string(),
    };

    let can_clear_player = session_status_v.play_status.is_player;
    let can_clear_publisher = session_status_v.is_publisher;

    let is_play_stream = stream_id == session_status_v.play_status.play_stream_id;

    if is_play_stream {
        session_status_v.play_status.play_stream_id = 0;
    }

    let is_publish_stream = stream_id == session_status_v.publish_stream_id;

    if is_publish_stream {
        session_status_v.publish_stream_id = 0;
    }

    drop(session_status_v);

    if is_play_stream {
        log_info!(logger, "PLAY STOP");

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
            log_debug!(
                logger,
                format!("Send error: Could not send status message: {}", e)
            );
        }

        if can_clear_player {
            remove_player(server_context, &channel, session_context.id).await;
            try_clear_channel(server_context, &channel).await;
        }
    }

    if is_publish_stream {
        log_info!(logger, "PUBLISH END");

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
            log_debug!(
                logger,
                format!("Send error: Could not send status message: {}", e)
            );
        }

        if can_clear_publisher {
            remove_publisher(logger, server_context, &channel, session_context.id).await;
            try_clear_channel(server_context, &channel).await;
        }
    }

    true
}
