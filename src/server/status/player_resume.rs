use crate::{
    server::RtmpServerContext,
    session::{RtmpSessionMessage, RtmpSessionPublishStreamStatus},
};

/// Resumes a player
///
/// # Arguments
///
/// * `server_context` - The server context
/// * `channel` - The channel ID
/// * `player_id` - ID of the player
pub async fn player_resume(server_context: &RtmpServerContext, channel: &str, player_id: u64) {
    let mut status = server_context.status.lock().await;

    if let Some(c) = status.channels.get_mut(channel) {
        let channel_mu = c.clone();
        drop(status);

        let mut channel_status = channel_mu.lock().await;

        let publishing = channel_status.publishing;
        let publish_status = channel_status.publish_status.clone();

        if let Some(player_status) = channel_status.players.get_mut(&player_id) {
            if !player_status.paused {
                return; // Not paused
            }

            player_status.paused = false;

            if publishing {
                if let Some(publish_status) = &publish_status {
                    let player_resume_message =
                        RtmpSessionPublishStreamStatus::get_player_resume_message(publish_status)
                            .await;

                    _ = player_status.message_sender.send(player_resume_message);
                } else {
                    _ = player_status
                        .message_sender
                        .send(RtmpSessionMessage::ResumeIdle)
                        .await;
                }
            } else {
                _ = player_status
                    .message_sender
                    .send(RtmpSessionMessage::ResumeIdle)
                    .await;
            }
        }
    }
}
