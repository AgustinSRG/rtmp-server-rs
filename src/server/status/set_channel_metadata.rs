use std::sync::Arc;

use crate::{
    server::RtmpServerContext,
    session::{RtmpSessionMessage, RtmpSessionPublishStreamStatus},
};

/// Sets channel metadata
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `channel` - The channel ID
/// * `publisher_id` - ID of the publisher setting the metadata
/// * `metadata` - The metadata
pub async fn set_channel_metadata(
    server_context: &RtmpServerContext,
    channel: &str,
    publisher_id: u64,
    metadata: Arc<Vec<u8>>,
) {
    let mut status = server_context.status.lock().await;

    if let Some(c) = status.channels.get_mut(channel) {
        let channel_mu = c.clone();
        drop(status);

        let channel_status = channel_mu.lock().await;

        if let Some(pid) = channel_status.publisher_id {
            if pid != publisher_id {
                return; // Not the publisher session
            }
        }

        let publish_status = match &channel_status.publish_status {
            Some(s) => s,
            None => {
                return;
            }
        };

        RtmpSessionPublishStreamStatus::set_metadata(publish_status, metadata.clone()).await;

        // Send metadata to players

        for player in channel_status.players.values() {
            _ = player
                .message_sender
                .send(RtmpSessionMessage::PlayMetadata {
                    metadata: metadata.clone(),
                })
                .await;
        }
    }
}
