// Messages to communicate between sessions

use std::sync::Arc;

use crate::rtmp::RtmpPacket;


/// RTMP session message
#[derive(Clone)]
pub enum RtmpSessionMessage {
    PlayStart{
        metadata: Arc<Vec<u8>>,
        audio_codec: u32,
        aac_sequence_header: Arc<Vec<u8>>,
        video_codec: u32,
        avc_sequence_header: Arc<Vec<u8>>,
        gop_cache: Vec<Arc<RtmpPacket>>,
    },
    PlayMetadata{
        metadata: Arc<Vec<u8>>,
    },
    PlayPacket{
        packet: Arc<RtmpPacket>,
    },
    PlayStop,
    InvalidKey,
}
