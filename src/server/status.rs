// Server status model

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use crate::session::{RtmpSessionMessage, RtmpSessionPublishStreamStatus};

/// Status of an RTMP player
pub struct RtmpPlayerStatus {
    /// Provided stream key
    pub provided_key: String,

    /// Message sender to communicate with the player session
    pub message_sender: Sender<RtmpSessionMessage>,

    /// True if GOP cache is enabled for the player
    pub gop_enabled: bool,

    /// True if the player wishes to clear the GOP cache
    pub gop_clear: bool,

    /// True if paused
    pub paused: bool,

    /// True to receive audio
    pub receive_audio: bool,

    /// True to receive video
    pub receive_video: bool,
}

/// Channel status
pub struct RtmpChannelStatus {
    /// Channel key
    pub key: Option<String>,

    /// Current stream ID
    pub stream_id: Option<String>,

    /// True if publishing
    pub publishing: bool,

    /// ID of the publisher session
    pub publisher_id: Option<u64>,

    /// Message sender for the publisher session
    pub publisher_message_sender: Option<Sender<RtmpSessionMessage>>,

    /// Status of the published stream
    pub publish_status: Option<Arc<Mutex<RtmpSessionPublishStreamStatus>>>,

    /// Players
    pub players: HashMap<u64, RtmpPlayerStatus>,
}

/// Server status
pub struct RtmpServerStatus {
    /// Channels
    pub channels: HashMap<String, RtmpChannelStatus>,
}

impl RtmpServerStatus {
    /// Creates new RtmpServerStatus
    pub fn new() -> RtmpServerStatus {
        RtmpServerStatus {
            channels: HashMap::new(),
        }
    }
}
