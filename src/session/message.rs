// Messages to communicate between sessions

use std::sync::Arc;

use crate::rtmp::RtmpPacket;

/// Size of the buffer for the message channel
pub const RTMP_SESSION_MESSAGE_BUFFER_SIZE: usize = 8;

/// RTMP session message
#[derive(Clone)]
pub enum RtmpSessionMessage {
    /// Message to start playing a stream
    PlayStart {
        metadata: Arc<Vec<u8>>,
        audio_codec: u32,
        aac_sequence_header: Arc<Vec<u8>>,
        video_codec: u32,
        avc_sequence_header: Arc<Vec<u8>>,
        gop_cache: Vec<Arc<RtmpPacket>>,
    },

    /// Message to send the metadata of the stream to play
    PlayMetadata {
        metadata: Arc<Vec<u8>>,
    },

    /// Message to send a packet of the stream to play
    PlayPacket {
        packet: Arc<RtmpPacket>,
    },

    /// Message to pause the stream being played
    Pause,

    /// Message to resume playing the stream
    Resume {
        audio_codec: u32,
        aac_sequence_header: Arc<Vec<u8>>,
        video_codec: u32,
        avc_sequence_header: Arc<Vec<u8>>,
    },

    /// Message to resume playing, but as Idle status
    ResumeIdle,

    /// Message to stop playing the stream
    PlayStop,

    /// Message to indicate an invalid key was given to play the stream
    InvalidKey,

    /// Message to kill the session
    Kill,

    /// Message sent at the end of the read thread
    End,
}
