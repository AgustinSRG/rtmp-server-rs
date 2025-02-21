use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    server::{RtmpChannelStatus, RtmpPlayerStatus, RtmpServerContext},
    session::SessionReadThreadContext,
    utils::string_compare_constant_time,
};

/// Options to add a player to a channel
pub struct AddPlayerOptions {
    /// Clear the GOP cache
    pub gop_clear: bool,

    /// Receive audio
    pub receive_audio: bool,

    /// Receive video
    pub receive_video: bool,
}

/// Adds a player to a channel
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `channel` - Channel ID
/// * `key` - Channel key
/// * `player_options` - The player options
///
/// # Return value
///
/// Returns true if success, false if cannot add the player (invalid key)
pub async fn add_player(
    server_context: &RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    channel: &str,
    key: &str,
    player_options: AddPlayerOptions,
) -> bool {
    let mut status = server_context.status.lock().await;

    match status.channels.get_mut(channel) {
        Some(c) => {
            let channel_mu = c.clone();
            drop(status);

            let mut channel_status = channel_mu.lock().await;

            let player_status = RtmpPlayerStatus {
                provided_key: key.to_string(),
                message_sender: session_context.session_msg_sender.clone(),
                gop_clear: player_options.gop_clear,
                paused: false,
                idle: !channel_status.publishing,
                receive_audio: player_options.receive_audio,
                receive_video: player_options.receive_video,
            };

            channel_status
                .players
                .insert(session_context.id, player_status);

            if !channel_status.publishing {
                // Not publishing yet, stay idle until a publisher appears
                return true;
            }

            if let Some(channel_key) = &channel_status.key {
                if !string_compare_constant_time(channel_key, key) {
                    // If the key is invalid, remove the player
                    channel_status.players.remove(&session_context.id);
                    return false;
                }
            }

            let publish_status_mu = match &channel_status.publish_status {
                Some(s) => s,
                None => {
                    return true;
                }
            };

            // Send the start message to the new player

            let mut publish_status = publish_status_mu.lock().await;

            let player_start_msg = publish_status.get_play_start_message();

            if player_options.gop_clear {
                publish_status.clear_gop();
            }

            drop(publish_status);

            _ = session_context
                .session_msg_sender
                .send(player_start_msg)
                .await;

            true
        }
        None => {
            let mut new_channel_status = RtmpChannelStatus::new();

            let player_status = RtmpPlayerStatus {
                provided_key: key.to_string(),
                message_sender: session_context.session_msg_sender.clone(),
                gop_clear: player_options.gop_clear,
                paused: false,
                idle: true,
                receive_audio: player_options.receive_audio,
                receive_video: player_options.receive_video,
            };

            new_channel_status
                .players
                .insert(session_context.id, player_status);

            let channel_mu = Arc::new(Mutex::new(new_channel_status));

            status.channels.insert(channel.to_string(), channel_mu);

            // Since this channel is brand new, no publishing, so the player remains idle

            true
        }
    }
}
