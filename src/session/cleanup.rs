// Session cleanup logic

use tokio::sync::Mutex;

use crate::{
    log::Logger,
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::RtmpSessionStatus;

/// Performs session cleanup
/// logger - The logger
/// session_id - Session ID
/// config - Server configuration
/// server_status - Server status
/// session_status - Session status
pub async fn do_session_cleanup(
    logger: &Logger,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
) {
    let session_status_v = session_status.lock().await;

    let channel = match &session_status_v.channel {
        Some(c) => c.clone(),
        None => {
            return; // Not connected yet, nothing to do
        }
    };

    let must_clear_player = session_status_v.is_player;
    let must_clear_publisher = session_status_v.is_publisher;

    drop(session_status_v);

    if must_clear_player {
        RtmpServerStatus::remove_player(server_status, &channel, session_id).await;
    }

    if must_clear_publisher {
        RtmpServerStatus::remove_publisher(logger, config, server_status, &channel, session_id)
            .await
    }

    if must_clear_player || must_clear_publisher {
        RtmpServerStatus::try_clear_channel(server_status, &channel).await;
    }
}
