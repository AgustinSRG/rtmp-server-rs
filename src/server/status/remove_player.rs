use crate::server::RtmpServerContext;

/// Removes a player from a channel
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `channel` - Channel ID
/// * `player_id` - The ID of the player to remove
pub async fn remove_player(server_context: &RtmpServerContext, channel: &str, player_id: u64) {
    let mut status = server_context.status.lock().await;

    if let Some(c) = status.channels.get_mut(channel) {
        let channel_mu = c.clone();
        drop(status);

        let mut channel_status = channel_mu.lock().await;

        channel_status.players.remove(&player_id);
    }
}
