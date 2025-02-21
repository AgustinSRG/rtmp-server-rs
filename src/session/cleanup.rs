// Session cleanup logic

use crate::{
    log::Logger,
    server::{remove_player, remove_publisher, try_clear_channel, RtmpServerContext},
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

    let must_clear_player = session_status_v.play_status.is_player;
    let must_clear_publisher = session_status_v.is_publisher;

    drop(session_status_v);

    if must_clear_player {
        remove_player(server_context, &channel, session_context.id).await;
    }

    if must_clear_publisher {
        remove_publisher(logger, server_context, &channel, session_context.id).await
    }

    if must_clear_player || must_clear_publisher {
        try_clear_channel(server_context, &channel).await;
    }
}
