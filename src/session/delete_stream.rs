// Stream deletion util

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{send_status_message, RtmpSessionStatus};

/// Deletes a stream from the RTMP session
/// packet - The packet to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn rtmp_delete_stream<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    stream_id: u32,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    let mut session_status_v = session_status.lock().await;

    let channel = match &session_status_v.channel {
        Some(c) => c.clone(),
        None => {
            if config.log_requests && logger.config.debug_enabled {
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
        if config.log_requests {
            logger.log_info("PLAY STOP");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            stream_id,
            "status",
            "NetStream.Play.Stop",
            Some("Stopped playing stream."),
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

        if can_clear_player {
            RtmpServerStatus::remove_player(server_status, &channel, session_id).await;
            RtmpServerStatus::try_clear_channel(server_status, &channel).await;
        }
    }

    if is_publish_stream {
        if config.log_requests {
            logger.log_info("PUBLISH END");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            stream_id,
            "status",
            "NetStream.Unpublish.Success",
            Some(&format!("/{}/{} is now unpublished.", channel, key)),
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

        if can_clear_publisher {
            RtmpServerStatus::remove_publisher(logger, config, server_status, &channel, session_id).await;
            RtmpServerStatus::try_clear_channel(server_status, &channel).await;
        }
    }

    true
}
