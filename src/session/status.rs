// RTMP session status model

use std::{collections::VecDeque, sync::Arc};

use chrono::Utc;
use tokio::sync::Mutex;

use crate::{
    rtmp::{RtmpPacket, RTMP_CHUNK_SIZE},
    server::RtmpChannelStatus,
};

use super::RtmpSessionMessage;

/// RTMP session status
pub struct RtmpSessionStatus {
    /// True if the session was killed
    pub killed: bool,

    /// True to receive audio
    pub receive_audio: bool,

    /// True to receive video
    pub receive_video: bool,

    /// Receive GOP cache?
    pub receive_gop: bool,

    /// Channel
    pub channel: Option<String>,

    /// Connect timestamp (Unix milliseconds)
    pub connect_time: i64,

    /// Key
    pub key: Option<String>,

    /// True if the session is player for the channel
    pub is_player: bool,

    /// ID of the RTMP stream used for playing
    pub play_stream_id: u32,

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
            receive_audio: true,
            receive_video: true,
            receive_gop: true,
            channel: None,
            connect_time: 0,
            key: None,
            is_publisher: false,
            publish_stream_id: 0,
            is_player: false,
            play_stream_id: 0,
            streams: 0,
        }
    }

    /// Checks if the session is a publisher
    pub async fn check_is_publisher(status: &Mutex<RtmpSessionStatus>) -> bool {
        let status_v = status.lock().await;
        status_v.is_publisher
    }

    /// Checks if the session is a player
    pub async fn check_is_player(status: &Mutex<RtmpSessionStatus>) -> bool {
        let status_v = status.lock().await;
        status_v.is_player
    }

    /// Checks if the session is killed
    pub async fn is_killed(status: &Mutex<RtmpSessionStatus>) -> bool {
        let status_v = status.lock().await;
        status_v.killed
    }

    /// Sets the session as killed
    pub async fn set_killed(status: &Mutex<RtmpSessionStatus>) {
        let mut status_v = status.lock().await;
        status_v.killed = true;
    }

    /// Updates session status for publishing
    pub async fn set_publisher(status: &Mutex<RtmpSessionStatus>, publish_stream_id: u32) {
        let mut status_v = status.lock().await;
        status_v.is_publisher = true;
        status_v.publish_stream_id = publish_stream_id;
    }

    /// Updates session status for playing
    /// Return the receive_audio, receive_video properties
    pub async fn set_player(
        status: &Mutex<RtmpSessionStatus>,
        receive_gop: bool,
        play_stream_id: u32,
    ) -> (bool, bool) {
        let mut status_v = status.lock().await;
        status_v.is_player = true;
        status_v.receive_gop = receive_gop;
        status_v.publish_stream_id = play_stream_id;

        (status_v.receive_audio, status_v.receive_video)
    }

    /// Gets the current channel of the session
    pub async fn get_channel(status: &Mutex<RtmpSessionStatus>) -> Option<String> {
        let status_v = status.lock().await;
        status_v.channel.clone()
    }

    /// Checks the play status of a session
    /// Return the is_player, play_stream_id, receive_gop, receive_audio, receive_video properties
    pub async fn check_play_status(
        status: &Mutex<RtmpSessionStatus>,
    ) -> (bool, u32, bool, bool, bool) {
        let status_v = status.lock().await;
        (
            status_v.is_player,
            status_v.play_stream_id,
            status_v.receive_gop,
            status_v.receive_audio,
            status_v.receive_video,
        )
    }

    /// Sets the playing status to false
    pub async fn stop_playing(status: &Mutex<RtmpSessionStatus>) {
        let mut status_v = status.lock().await;
        status_v.is_player = false;
    }

    /// Checks the play status of a session
    /// Return the is_player, play_stream_id properties
    pub async fn get_play_stream_id(status: &Mutex<RtmpSessionStatus>) -> (bool, u32) {
        let status_v = status.lock().await;
        (status_v.is_player, status_v.play_stream_id)
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

    /// Sets the clock value
    pub async fn set_clock(status_mu: &Mutex<RtmpSessionPublishStreamStatus>, clock_val: i64) {
        let mut status = status_mu.lock().await;
        status.clock = clock_val;
    }

    /// Sets the metadata value
    pub async fn set_metadata(
        status_mu: &Mutex<RtmpSessionPublishStreamStatus>,
        metadata: Arc<Vec<u8>>,
    ) {
        let mut status = status_mu.lock().await;
        status.metadata = metadata;
    }

    /// Gets message to wake players
    pub async fn get_play_start_message(
        status_mu: &Mutex<RtmpSessionPublishStreamStatus>,
        clear_gop: bool,
    ) -> RtmpSessionMessage {
        let mut status = status_mu.lock().await;

        let copy_of_gop_cache: Vec<Arc<RtmpPacket>> = status.gop_cache.iter().cloned().collect();

        let msg = RtmpSessionMessage::PlayStart {
            metadata: status.metadata.clone(),
            audio_codec: status.audio_codec,
            aac_sequence_header: status.aac_sequence_header.clone(),
            video_codec: status.video_codec,
            avc_sequence_header: status.avc_sequence_header.clone(),
            gop_cache: copy_of_gop_cache,
        };

        if clear_gop && !status.gop_cache_cleared {
            status.gop_cache.clear();
            status.gop_cache_cleared = true;
            status.gop_cache_size = 0;
        }

        msg
    }

    /// Gets message to wake players
    pub async fn get_player_resume_message(
        status_mu: &Mutex<RtmpSessionPublishStreamStatus>,
    ) -> RtmpSessionMessage {
        let status = status_mu.lock().await;

        RtmpSessionMessage::Resume {
            audio_codec: status.audio_codec,
            aac_sequence_header: status.aac_sequence_header.clone(),
            video_codec: status.video_codec,
            avc_sequence_header: status.avc_sequence_header.clone(),
        }
    }

    /// Pushes a new packet to the gop cache
    pub async fn push_new_packet(
        status_mu: &Mutex<RtmpSessionPublishStreamStatus>,
        packet: Arc<RtmpPacket>,
        gop_max_size: usize,
    ) {
        let mut status = status_mu.lock().await;

        let packet_size = packet.size();

        status.gop_cache_size = status.gop_cache_size.wrapping_add(packet_size);
        status.gop_cache.push_back(packet);

        while !status.gop_cache.is_empty() && status.gop_cache_size > gop_max_size {
            if let Some(removed) = status.gop_cache.pop_front() {
                status.gop_cache_size = status.gop_cache_size.wrapping_sub(removed.size());
            }
        }
    }
}
