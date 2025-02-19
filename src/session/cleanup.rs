// Session cleanup logic

use crate::{
    log::Logger,
    server::{RtmpServerContext, RtmpServerStatus},
};

use super::SessionContext;

/// Performs session cleanup
///
/// # Arguments
///
/// * `logger` - The server logger
/// * `server_context` - The server context
/// * `session_context` - The session context
pub async fn do_session_cleanup(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &SessionContext,
) {
    let session_status_v = session_context.status.lock().await;

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
        RtmpServerStatus::remove_player(&server_context.status, &channel, session_context.id).await;
    }

    if must_clear_publisher {
        RtmpServerStatus::remove_publisher(
            logger,
            &server_context.config,
            &server_context.status,
            &mut server_context.control_key_validator_sender,
            &channel,
            session_context.id,
        )
        .await
    }

    if must_clear_player || must_clear_publisher {
        RtmpServerStatus::try_clear_channel(&server_context.status, &channel).await;
    }
}
