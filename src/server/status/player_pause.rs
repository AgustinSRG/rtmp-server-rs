use crate::{server::RtmpServerContext, session::RtmpSessionMessage};

/// Pauses a player
/// 
/// # Arguments
/// 
/// * `server_context` - The server context
/// * `channel` - The channel ID
/// * `player_id` - ID of the player
pub async fn player_pause(server_context: &RtmpServerContext, channel: &str, player_id: u64) {
    let mut status = server_context.status.lock().await;

    if let Some(c) = status.channels.get_mut(channel) {
        let channel_mu = c.clone();
        drop(status);

        let mut channel_status = channel_mu.lock().await;

        if let Some(player_status) = channel_status.players.get_mut(&player_id) {
            if player_status.paused {
                return; // Already paused
            }

            player_status.paused = true;
            _ = player_status
                .message_sender
                .send(RtmpSessionMessage::Pause)
                .await;
        }
    }
}