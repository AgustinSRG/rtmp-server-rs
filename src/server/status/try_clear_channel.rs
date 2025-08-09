use crate::server::RtmpServerContext;

/// Tries to clear an unused channel
/// Call after every removal of a player or a publisher
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `channel` - The channel ID
pub async fn try_clear_channel(server_context: &RtmpServerContext, channel: &str) {
    let mut status = server_context.status.lock().await;

    let should_delete = match status.channels.get(channel) {
        Some(c) => {
            let channel_status = c.lock().await;
            !channel_status.publishing && channel_status.players.is_empty()
        }
        None => false,
    };

    if should_delete {
        status.channels.remove(channel);
    }
}
