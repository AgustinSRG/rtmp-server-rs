// Logic to handle audio packets

use std::sync::Arc;

use crate::{
    log::Logger,
    rtmp::{RtmpPacket, RTMP_CHANNEL_AUDIO, RTMP_CHUNK_TYPE_0, RTMP_TYPE_AUDIO},
    server::{RtmpServerContext, RtmpServerStatus},
};

use super::SessionReadThreadContext;

/// Handles AUDIO RTMP packet
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
pub async fn handle_rtmp_packet_audio(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    packet: &RtmpPacket,
) -> bool {
    let channel_status = match &session_context.read_status.channel_status {
        Some(s) => s,
        None => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Audio packet ignored since it was not publishing");
            }

            return true;
        }
    };

    if packet.header.length <= 3 {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Packet error: Packet length too short");
        }

        return false;
    }

    // Load packet metadata and update publish status

    let mut publish_status_v = session_context.publish_status.lock().await;

    let sound_format = (packet.payload[0] >> 4) & 0x0f;

    if publish_status_v.audio_codec == 0 {
        publish_status_v.audio_codec = sound_format as u32;
    }

    let is_header = (sound_format == 10 || sound_format == 13) && packet.payload[1] == 0;

    if is_header {
        publish_status_v.aac_sequence_header = Arc::new(packet.payload.clone());
    }

    let clock = publish_status_v.clock;

    drop(publish_status_v);

    // Log

    if server_context.config.log_requests && logger.config.trace_enabled {
        logger.log_trace(&format!("AUDIO PACKET: {} bytes", packet.payload.len()));
    }

    // Prepare packet copy to store

    let mut copied_packet = RtmpPacket::new_blank();

    copied_packet.header.format = RTMP_CHUNK_TYPE_0;
    copied_packet.header.channel_id = RTMP_CHANNEL_AUDIO;
    copied_packet.header.packet_type = RTMP_TYPE_AUDIO;
    copied_packet.payload = packet.payload.clone();
    copied_packet.header.length = copied_packet.payload.len();
    copied_packet.header.timestamp = clock;

    // Send packet to the channel

    RtmpServerStatus::send_packet_to_channel(
        channel_status,
        session_context.id,
        Arc::new(copied_packet),
        is_header,
        &server_context.config,
    )
    .await;

    // Done

    true
}
