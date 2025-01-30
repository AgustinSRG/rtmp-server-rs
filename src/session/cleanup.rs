// Session cleanup logic

use tokio::sync::Mutex;

use crate::server::RtmpServerStatus;

use super::RtmpSessionStatus;

/// Performs session cleanup
/// session_id - Session ID
/// server_status - Server status
/// session_status - Session status
pub async fn do_session_cleanup(
    session_id: u64,
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
        RtmpServerStatus::remove_publisher(server_status, &channel, session_id).await
    }

    if must_clear_player || must_clear_publisher {
        RtmpServerStatus::try_clear_channel(server_status, &channel).await;
    }
}
