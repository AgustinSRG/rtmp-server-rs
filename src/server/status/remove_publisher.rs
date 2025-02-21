use crate::{callback::make_stop_callback, control::ControlKeyValidationRequest, log::Logger, server::RtmpServerContext, session::RtmpSessionMessage};


/// Removes a publisher from a channel
/// 
/// # Arguments
/// 
/// * `logger` - The logger
/// * `server_context` - The server context
/// * `channel` - The channel ID
/// * `publisher_id` - ID of the publisher to remove
pub async fn remove_publisher(
    logger: &Logger,
    server_context: &RtmpServerContext,
    channel: &str,
    publisher_id: u64,
) {
    let status = server_context.status.lock().await;

    if let Some(c) = status.channels.get(channel) {
        let channel_mu = c.clone();
        drop(status);

        let mut channel_status = channel_mu.lock().await;

        if !channel_status.publishing {
            return;
        }

        if let Some(pid) = channel_status.publisher_id {
            if pid != publisher_id {
                return; // Not the publisher session
            }
        }

        // Unpublish

        let unpublished_stream_key = match &channel_status.key {
            Some(k) => k.clone(),
            None => "".to_string(),
        };

        let unpublished_stream_id = match &channel_status.stream_id {
            Some(i) => i.clone(),
            None => "".to_string(),
        };

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

        drop(channel_status);

        // Send callback

        match &server_context.control_key_validator_sender {
            Some(sender) => {
                // Notify control server
                _ = sender
                    .send(ControlKeyValidationRequest::PublishEnd {
                        channel: channel.to_string(),
                        stream_id: unpublished_stream_id,
                    })
                    .await;
            }
            None => {
                // Callback
                make_stop_callback(
                    logger,
                    &server_context.config.callback,
                    channel,
                    &unpublished_stream_key,
                    &unpublished_stream_id,
                )
                .await;
            }
        }
    }
}