use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use crate::{rtmp::{RtmpPacket, RTMP_TYPE_AUDIO, RTMP_TYPE_VIDEO}, session::{RtmpSessionMessage, RtmpSessionPublishStreamStatus}};

/// Status of an RTMP player
pub struct RtmpPlayerStatus {
    /// Provided stream key
    pub provided_key: String,

    /// Message sender to communicate with the player session
    pub message_sender: Sender<RtmpSessionMessage>,

    /// True if the player wishes to clear the GOP cache
    pub gop_clear: bool,

    /// True if paused
    pub paused: bool,

    /// True if idle
    pub idle: bool,

    /// True to receive audio
    pub receive_audio: bool,

    /// True to receive video
    pub receive_video: bool,
}

/// RTMP channel status
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

impl RtmpChannelStatus {
    /// Creates a new instance of RtmpChannelStatus
    pub fn new() -> RtmpChannelStatus {
        RtmpChannelStatus {
            publishing: false,
            key: None,
            stream_id: None,
            publisher_id: None,
            publisher_message_sender: None,
            publish_status: None,
            players: HashMap::new(),
        }
    }

    /// Sends a packet to players and stored it in the GOP cache if applicable
    /// 
    /// # Arguments
    /// 
    /// * `publisher_id` - ID of the publisher sending the packet
    /// * `packet` - Packet to send
    /// * `skip_cache` - True if the packet should not be added to the GOP cache
    /// * `gop_cache_size` - The max size of the GOP cache (server config)
    pub async fn send_packet(
        &self,
        publisher_id: u64,
        packet: Arc<RtmpPacket>,
        skip_cache: bool,
        gop_cache_size: usize,
    ) {
        if !self.publishing {
            return;
        }

        if let Some(pid) = self.publisher_id {
            if pid != publisher_id {
                return; // Not the publisher session
            }
        }

        let publish_status = match &self.publish_status {
            Some(s) => s,
            None => {
                return;
            }
        };

        if !skip_cache {
            RtmpSessionPublishStreamStatus::push_new_packet(
                publish_status,
                packet.clone(),
                gop_cache_size,
            )
            .await;
        }

        // Send packet to players

        for player in self.players.values() {
            if player.paused {
                continue;
            }

            if packet.header.packet_type == RTMP_TYPE_AUDIO && !player.receive_audio {
                continue;
            }

            if packet.header.packet_type == RTMP_TYPE_VIDEO && !player.receive_video {
                continue;
            }

            _ = player
                .message_sender
                .send(RtmpSessionMessage::PlayPacket {
                    packet: packet.clone(),
                })
                .await;
        }
    }
}
