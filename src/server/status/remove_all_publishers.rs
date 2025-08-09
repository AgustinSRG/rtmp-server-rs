use crate::{server::RtmpServerContext, session::RtmpSessionMessage};

/// Removes all the publishers and kills them
///
/// # Arguments
///
/// * `server_context` - Server context
pub async fn remove_all_publishers(server_context: &RtmpServerContext) {
    let mut status = server_context.status.lock().await;

    let mut channels_to_delete: Vec<String> = Vec::new();

    for (channel, c) in &mut status.channels {
        let mut channel_status = c.lock().await;

        if !channel_status.publishing {
            continue;
        }

        // Kill the publisher

        if let Some(pub_sender) = &channel_status.publisher_message_sender {
            _ = pub_sender.send(RtmpSessionMessage::Kill).await;
        }

        // Unpublish

        channel_status.publishing = false;
        channel_status.publisher_id = None;
        channel_status.publish_status = None;
        channel_status.publisher_message_sender = None;
        channel_status.key = None;
        channel_status.stream_id = None;

        // Notify players

        for player in channel_status.players.values_mut() {
            player.idle = true;
            _ = player
                .message_sender
                .send(RtmpSessionMessage::PlayStop)
                .await;
        }

        // Check if it can be deleted

        if channel_status.players.is_empty() {
            channels_to_delete.push(channel.clone());
        }
    }

    // Remove empty channels

    for channel in channels_to_delete {
        status.channels.remove(&channel);
    }
}
