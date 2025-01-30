// Logic to handle video packets

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    log::Logger,
    rtmp::{RtmpPacket, RTMP_CHANNEL_VIDEO, RTMP_CHUNK_TYPE_0, RTMP_TYPE_VIDEO},
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{RtmpSessionPublishStreamStatus, RtmpSessionReadStatus};

/// Handles RTMP packet (VIDEO)
/// packet - The packet to handle
/// session_id - Session ID
/// config - RTMP configuration
/// publish_status - Status if the stream being published
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunk. Returns false to end the session main loop.
pub async fn handle_rtmp_packet_video(
    packet: &RtmpPacket,
    session_id: u64,
    config: &RtmpServerConfiguration,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    read_status: &mut RtmpSessionReadStatus,
    logger: &Logger,
) -> bool {
    let channel_status = match &read_status.channel_status {
        Some(s) => s,
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Audio packet ignored since it was not publishing");
            }

            return true;
        }
    };

    if packet.header.length <= 3 {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Packet error: Packet length too short");
        }

        return false;
    }

    // Load packet metadata and update publish status

    let mut publish_status_v = publish_status.lock().await;

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

    // Prepare packet copy to store

    let mut copied_packet = RtmpPacket::new_blank();

    copied_packet.header.format = RTMP_CHUNK_TYPE_0;
    copied_packet.header.channel_id = RTMP_CHANNEL_VIDEO;
    copied_packet.header.packet_type = RTMP_TYPE_VIDEO;
    copied_packet.payload = packet.payload.clone();
    copied_packet.header.length = copied_packet.payload.len();
    copied_packet.header.timestamp = clock;

    // Send packet to the channel

    RtmpServerStatus::send_packet_to_channel(
        &channel_status,
        session_id,
        Arc::new(copied_packet),
        is_header,
        config,
    )
    .await;

    // Done

    true
}
