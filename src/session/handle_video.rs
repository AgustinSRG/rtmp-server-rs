// Logic to handle video packets

use std::sync::Arc;

use crate::{
    log::Logger,
    log_debug, log_trace,
    rtmp::{RtmpPacket, RTMP_CHANNEL_VIDEO, RTMP_CHUNK_TYPE_0, RTMP_TYPE_VIDEO},
    server::RtmpServerContext,
};

use super::SessionReadThreadContext;

/// Handles VIDEO RTMP packet
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `packet` - The packet
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet_video(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    packet: &RtmpPacket,
) -> bool {
    let channel_status_mu = match &session_context.read_status.channel_status {
        Some(s) => s,
        None => {
            log_debug!(logger, "Audio packet ignored since it was not publishing");

            return true;
        }
    };

    if packet.header.length <= 3 {
        log_debug!(logger, "Packet error: Packet length too short");

        return false;
    }

    // Load packet metadata and update publish status

    let mut publish_status_v = session_context.publish_status.lock().await;

    let frame_type = (packet.payload[0] >> 4) & 0x0f;
    let codec_id = packet.payload[0] & 0x0f;

    let is_header =
        (codec_id == 7 || codec_id == 12) && (frame_type == 1 && packet.payload[1] == 0);

    if is_header {
        publish_status_v.avc_sequence_header = Arc::new(packet.payload.clone());
        publish_status_v.gop_cache.clear();
        publish_status_v.gop_cache_size = 0;
    }

    if publish_status_v.video_codec == 0 {
        publish_status_v.video_codec = codec_id as u32;
    }

    let clock = publish_status_v.clock;

    drop(publish_status_v);

    // Log

    log_trace!(
        logger,
        format!("VIDEO PACKET: {} bytes", packet.payload.len())
    );

    // Prepare packet copy to store

    let mut copied_packet = RtmpPacket::new_blank();

    copied_packet.header.format = RTMP_CHUNK_TYPE_0;
    copied_packet.header.channel_id = RTMP_CHANNEL_VIDEO;
    copied_packet.header.packet_type = RTMP_TYPE_VIDEO;
    copied_packet.payload = packet.payload.clone();
    copied_packet.header.length = copied_packet.payload.len();
    copied_packet.header.timestamp = clock;

    // Send packet to the channel

    let channel_status = channel_status_mu.lock().await;

    channel_status
        .send_packet(
            session_context.id,
            Arc::new(copied_packet),
            is_header,
            server_context.config.gop_cache_size,
        )
        .await;

    drop(channel_status);

    // Done

    true
}
