// Messages to communicate between sessions

use std::sync::Arc;

use tokio::{io::{AsyncRead, AsyncReadExt}, sync::mpsc::Receiver};

use crate::{log::Logger, rtmp::RtmpPacket};

/// Size of the buffer for the message channel
pub const RTMP_SESSION_MESSAGE_BUFFER_SIZE: usize = 8;

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
    End,
}
