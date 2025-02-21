// RTMP session status model

use std::{collections::VecDeque, sync::Arc};

use chrono::Utc;
use tokio::sync::Mutex;

use crate::{
    rtmp::{RtmpPacket, RTMP_CHUNK_SIZE},
    server::RtmpChannelStatus,
};

use super::RtmpSessionMessage;

/// Status of the session playing a stream
#[derive(Clone)]
pub struct RtmpSessionPlayStatus {
    /// True if the session is player for the channel
    pub is_player: bool,

    /// ID of the RTMP stream used for playing
    pub play_stream_id: u32,

    /// True to receive audio
    pub receive_audio: bool,

    /// True to receive video
    pub receive_video: bool,

    /// Receive GOP cache?
    pub receive_gop: bool,
}

impl RtmpSessionPlayStatus {
    /// Creates new instance of RtmpSessionPlayStatus
    pub fn new() -> RtmpSessionPlayStatus {
        RtmpSessionPlayStatus {
            is_player: false,
            play_stream_id: 0,
            receive_audio: true,
            receive_video: true,
            receive_gop: true,
        }
    }
}

/// RTMP session status
pub struct RtmpSessionStatus {
    /// Connect timestamp (Unix milliseconds)
    pub connect_time: i64,

    /// True if the session was killed
    pub killed: bool,

    /// Channel
    pub channel: Option<String>,

    /// Key
    pub key: Option<String>,

    /// The player status
    pub play_status: RtmpSessionPlayStatus,

    /// True if the session is a publisher for a channel
    pub is_publisher: bool,

    /// ID of the RTMP stream used for publishing
    pub publish_stream_id: u32,

    /// Current number of streams
    pub streams: usize,
}

impl RtmpSessionStatus {
    /// Creates new RtmpSessionStatus
    pub fn new() -> RtmpSessionStatus {
        RtmpSessionStatus {
            killed: false,
            channel: None,
            connect_time: 0,
            key: None,
            play_status: RtmpSessionPlayStatus::new(),
            is_publisher: false,
            publish_stream_id: 0,
            streams: 0,
        }
    }
}

/// Status to maintain only for the read task
pub struct RtmpSessionReadStatus {
    /// Size for incoming chunks
    pub in_chunk_size: usize,

    /// Size for ACKs
    pub in_ack_size: usize,

    /// Last ACK size
    pub in_last_ack: usize,

    /// ACK size
    pub ack_size: usize,

    /// Bit rate bytes counter
    pub bit_rate_bytes: usize,

    /// Bitrate last updated (Unix milliseconds)
    pub bit_rate_last_update: i64,

    /// Channel status (set only when publishing)
    pub channel_status: Option<Arc<Mutex<RtmpChannelStatus>>>,
}

impl RtmpSessionReadStatus {
    /// Creates RtmpSessionReadStatus
    pub fn new() -> RtmpSessionReadStatus {
        RtmpSessionReadStatus {
            in_chunk_size: RTMP_CHUNK_SIZE,
            in_ack_size: 0,
            in_last_ack: 0,
            ack_size: 0,
            bit_rate_bytes: 0,
            bit_rate_last_update: Utc::now().timestamp_millis(),
            channel_status: None,
        }
    }
}

/// Status of the stream being published
pub struct RtmpSessionPublishStreamStatus {
    /// Clock value
    pub clock: i64,

    /// Video codec
    pub video_codec: u32,

    /// AVC sequence header
    pub avc_sequence_header: Arc<Vec<u8>>,

    /// Audio codec
    pub audio_codec: u32,

    /// AAC sequence header
    pub aac_sequence_header: Arc<Vec<u8>>,

    /// Metadata
    pub metadata: Arc<Vec<u8>>,

    /// GOP cache
    pub gop_cache: VecDeque<Arc<RtmpPacket>>,

    /// GOP cache clear flag
    pub gop_cache_cleared: bool,

    /// Size of the GOP cache
    pub gop_cache_size: usize,
}

impl RtmpSessionPublishStreamStatus {
    /// Creates new RtmpSessionPublishStreamStatus
    pub fn new() -> RtmpSessionPublishStreamStatus {
        RtmpSessionPublishStreamStatus {
            clock: 0,
            audio_codec: 0,
            aac_sequence_header: Arc::new(Vec::new()),
            video_codec: 0,
            avc_sequence_header: Arc::new(Vec::new()),
            metadata: Arc::new(Vec::new()),
            gop_cache: VecDeque::new(),
            gop_cache_cleared: false,
            gop_cache_size: 0,
        }
    }

    /// Gets message to wake players
    pub fn get_play_start_message(&self) -> RtmpSessionMessage {
        let copy_of_gop_cache: Vec<Arc<RtmpPacket>> = self.gop_cache.iter().cloned().collect();

        RtmpSessionMessage::PlayStart {
            metadata: self.metadata.clone(),
            audio_codec: self.audio_codec,
            aac_sequence_header: self.aac_sequence_header.clone(),
            video_codec: self.video_codec,
            avc_sequence_header: self.avc_sequence_header.clone(),
            gop_cache: copy_of_gop_cache,
        }
    }

    /// Clears the GOP cache
    pub fn clear_gop(&mut self) {
        if !self.gop_cache_cleared {
            self.gop_cache.clear();
            self.gop_cache_cleared = true;
            self.gop_cache_size = 0;
        }
    }

    /// Gets message to resume players
    pub fn get_player_resume_message(&self) -> RtmpSessionMessage {
        RtmpSessionMessage::Resume {
            audio_codec: self.audio_codec,
            aac_sequence_header: self.aac_sequence_header.clone(),
            video_codec: self.video_codec,
            avc_sequence_header: self.avc_sequence_header.clone(),
        }
    }
}
