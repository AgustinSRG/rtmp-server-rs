use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    server::{RtmpChannelStatus, RtmpServerContext},
    session::{RtmpSessionMessage, SessionReadThreadContext},
    utils::string_compare_constant_time,
};

/// Sets a publisher for a channel
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `channel` - Channel ID
/// * `key` - Channel key
/// * `stream_id` - Stream ID
///
/// # Return value
///
/// Returns true if success, false if already publishing
pub async fn set_publisher(
    server_context: &RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    channel: &str,
    key: &str,
    stream_id: &str,
) -> bool {
    let channel_status_ref: Arc<Mutex<RtmpChannelStatus>>;

    let mut status = server_context.status.lock().await;

    match status.channels.get(channel) {
        Some(channel_mu) => {
            let channel_mu_clone = channel_mu.clone();
            channel_status_ref = channel_mu.clone();
            drop(status);

            let mut c = channel_mu_clone.lock().await;

            if c.publishing {
                return false;
            }

            // Update
            c.key = Some(key.to_string());
            c.stream_id = Some(stream_id.to_string());
            c.publishing = true;
            c.publisher_id = Some(session_context.id);
            c.publish_status = Some(session_context.publish_status.clone());
            c.publisher_message_sender = Some(session_context.session_msg_sender.clone());

            // Get idle players

            let mut players_to_remove: Vec<u64> = Vec::new();

            for (player_id, player) in &mut c.players {
                if player.idle {
                    if string_compare_constant_time(&player.provided_key, key) {
                        // Correct key, start player

                        let mut publish_status = session_context.publish_status.lock().await;

                        let play_start_message = publish_status.get_play_start_message();

                        if player.gop_clear {
                            publish_status.clear_gop();
                        }

                        drop(publish_status);

                        _ = player.message_sender.send(play_start_message);
                    } else {
                        // Invalid key
                        players_to_remove.push(*player_id);
                        _ = player.message_sender.send(RtmpSessionMessage::InvalidKey);
                    }

                    player.idle = false;
                }
            }

            for player_to_remove in players_to_remove {
                c.players.remove(&player_to_remove);
            }
        }
        None => {
            let mut new_channel_status = RtmpChannelStatus::new();

            new_channel_status.key = Some(key.to_string());
            new_channel_status.stream_id = Some(stream_id.to_string());
            new_channel_status.publishing = true;
            new_channel_status.publisher_id = Some(session_context.id);
            new_channel_status.publish_status = Some(session_context.publish_status.clone());
            new_channel_status.publisher_message_sender =
                Some(session_context.session_msg_sender.clone());

            let channel_mu = Arc::new(Mutex::new(new_channel_status));

            channel_status_ref = channel_mu.clone();

            status.channels.insert(channel.to_string(), channel_mu);

            drop(status)
        }
    };

    session_context.read_status.channel_status = Some(channel_status_ref);

    true
}
