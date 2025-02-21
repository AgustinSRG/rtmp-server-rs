// Server utils

use super::RtmpServerContext;

/// Checks publishing status of a channel
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `channel` - The channel ID
///
/// # Return value
///
/// Returns true if the channel is being published,
/// false if the channel is free to publish
pub async fn check_channel_publishing_status(
    server_context: &RtmpServerContext,
    channel: &str,
) -> bool {
    let status = server_context.status.lock().await;

    match status.channels.get(channel) {
        Some(c) => {
            let channel_mu = c.clone();
            drop(status);

            let channel_status = channel_mu.lock().await;

            channel_status.publishing
        }
        None => false,
    }
}
