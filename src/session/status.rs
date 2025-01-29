// RTMP session status model

use std::net::IpAddr;

use tokio::sync::Mutex;

use crate::rtmp::RTMP_CHUNK_SIZE;

/// RTMP session status
pub struct RtmpSessionStatus {
    /// Session ID
    pub id: u64,

    /// Client IP address
    pub ip: IpAddr,
}

impl RtmpSessionStatus {
    /// Creates new RtmpSessionStatus
    pub fn new(id: u64, ip: IpAddr) -> RtmpSessionStatus {
        RtmpSessionStatus { id, ip }
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
            bit_rate_last_update: 0,
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
    pub avc_sequence_header: Vec<u8>,

    /// Audio codec
    pub audio_codec: u32,

    /// AAC sequence header
    pub aac_sequence_header: Vec<u8>,
}

impl RtmpSessionPublishStreamStatus {
    /// Creates new RtmpSessionPublishStreamStatus
    pub fn new() -> RtmpSessionPublishStreamStatus {
        RtmpSessionPublishStreamStatus {
            clock: 0,
            audio_codec: 0,
            aac_sequence_header: Vec::new(),
            video_codec: 0,
            avc_sequence_header: Vec::new(),
        }
    }

    /// Sets the clock value
    pub async fn set_clock(status_mu: &Mutex<RtmpSessionPublishStreamStatus>, clock_val: i64) {
        let mut status = status_mu.lock().await;
        status.clock = clock_val;
    }
}
