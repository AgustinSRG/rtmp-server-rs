use crate::server::RtmpServerContext;

/// Sets receive audio option for a player
///
/// # Arguments
///
/// * `logger` - The logger
/// * `server_context` - The server context
/// * `channel` - The channel ID
/// * `player_id` - ID of the player
/// * `receive_audio` - Receive audio option
pub async fn player_set_receive_audio(
    server_context: &RtmpServerContext,
    channel: &str,
    player_id: u64,
    receive_audio: bool,
) {
    let mut status = server_context.status.lock().await;

    if let Some(c) = status.channels.get_mut(channel) {
        let channel_mu = c.clone();
        drop(status);

        let mut channel_status = channel_mu.lock().await;

        if let Some(player_status) = channel_status.players.get_mut(&player_id) {
            player_status.receive_audio = receive_audio;
        }
    }
}

/// Sets receive video option for a player
///
/// # Arguments
///
/// * `logger` - The logger
/// * `server_context` - The server context
/// * `channel` - The channel ID
/// * `player_id` - ID of the player
/// * `receive_video` - Receive video option
pub async fn player_set_receive_video(
    server_context: &RtmpServerContext,
    channel: &str,
    player_id: u64,
    receive_video: bool,
) {
    let mut status_v = server_context.status.lock().await;

    if let Some(c) = status_v.channels.get_mut(channel) {
        let channel_mu = c.clone();
        drop(status_v);

        let mut channel_status = channel_mu.lock().await;

        if let Some(player_status) = channel_status.players.get_mut(&player_id) {
            player_status.receive_video = receive_video;
        }
    }
}
