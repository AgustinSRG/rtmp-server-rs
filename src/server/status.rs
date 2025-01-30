// Server status model

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use crate::{
    rtmp::{RtmpPacket, RTMP_TYPE_AUDIO, RTMP_TYPE_VIDEO},
    session::{RtmpSessionMessage, RtmpSessionPublishStreamStatus},
    utils::string_compare_constant_time,
};

use super::RtmpServerConfiguration;

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

impl RtmpChannelStatus {
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
}

/// Server status
pub struct RtmpServerStatus {
    /// Channels
    pub channels: HashMap<String, Arc<Mutex<RtmpChannelStatus>>>,
}

impl RtmpServerStatus {
    /// Creates new RtmpServerStatus
    pub fn new() -> RtmpServerStatus {
        RtmpServerStatus {
            channels: HashMap::new(),
        }
    }

    /// Checks the publishing status of a channel
    pub async fn check_channel_publishing_status(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
    ) -> bool {
        let status_v = status.lock().await;

        match status_v.channels.get(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let channel_status = channel_mu.lock().await;

                return channel_status.publishing;
            }
            None => false,
        }
    }

    /// Sets a publisher for a channel
    /// Return true if success, or false if another session was publishing on the channel
    pub async fn set_publisher(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        key: &str,
        stream_id: &str,
        session_id: u64,
        publish_status: Arc<Mutex<RtmpSessionPublishStreamStatus>>,
        message_sender: Sender<RtmpSessionMessage>,
    ) -> bool {
        let mut status_v = status.lock().await;

        match status_v.channels.get(channel) {
            Some(channel_mu) => {
                let channel_mu_clone = channel_mu.clone();
                drop(status_v);

                let mut c = channel_mu_clone.lock().await;

                if c.publishing {
                    return false;
                }

                // Update
                c.key = Some(key.to_string());
                c.stream_id = Some(stream_id.to_string());
                c.publishing = true;
                c.publisher_id = Some(session_id);
                c.publish_status = Some(publish_status.clone());
                c.publisher_message_sender = Some(message_sender);

                // Get idle players

                let mut players_to_remove: Vec<u64> = Vec::new();

                for (player_id, player) in &mut c.players {
                    if player.idle {
                        if string_compare_constant_time(&player.provided_key, key) {
                            // Correct key, start player

                            let play_start_message =
                                RtmpSessionPublishStreamStatus::get_play_start_message(
                                    &publish_status,
                                    player.gop_clear,
                                )
                                .await;

                            _ = player.message_sender.send(play_start_message);
                        } else {
                            // Invalid key
                            players_to_remove.push(*player_id);
                            _ = player.message_sender.send(RtmpSessionMessage::InvalidKey);
                        }

                        player.idle = false;
                    }
                }

                for player_to_remove in players_to_remove {
                    c.players.remove(&player_to_remove);
                }
            }
            None => {
                let mut new_channel_status = RtmpChannelStatus::new();

                new_channel_status.key = Some(key.to_string());
                new_channel_status.stream_id = Some(stream_id.to_string());
                new_channel_status.publishing = true;
                new_channel_status.publisher_id = Some(session_id);
                new_channel_status.publish_status = Some(publish_status.clone());
                new_channel_status.publisher_message_sender = Some(message_sender);

                let channel_mu = Arc::new(Mutex::new(new_channel_status));

                status_v.channels.insert(channel.to_string(), channel_mu);
            }
        };

        true
    }

    /// Removes a player from a channel
    pub async fn remove_publisher(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        publisher_id: u64,
    ) {
        let status_v = status.lock().await;

        match status_v.channels.get(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let mut channel_status = channel_mu.lock().await;

                if !channel_status.publishing {
                    return;
                }

                if let Some(pid) = channel_status.publisher_id {
                    if pid == publisher_id {
                        return;
                    }
                }

                // Unpublish

                channel_status.publishing = false;
                channel_status.publisher_id = None;
                channel_status.publish_status = None;
                channel_status.publisher_message_sender = None;
                channel_status.key = None;
                channel_status.stream_id = None;

                // Notify players

                for (_, player) in &mut channel_status.players {
                    player.idle = true;
                    player.message_sender.send(RtmpSessionMessage::PlayStop);
                }
            }
            None => {
                return;
            }
        }
    }

    /// Tries to clear an unused channel
    /// Call after every removal of a player or a publisher
    pub async fn try_clear_channel(status: &Mutex<RtmpServerStatus>, channel: &str) {
        let mut status_v = status.lock().await;

        let should_delete = match status_v.channels.get(channel) {
            Some(c) => {
                let channel_status = c.lock().await;
                !channel_status.publishing && channel_status.players.is_empty()
            }
            None => false,
        };

        if should_delete {
            status_v.channels.remove(channel);
        }
    }

    /// Adds a player to a channel
    pub async fn add_player(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        key: &str,
        session_id: u64,
        message_sender: Sender<RtmpSessionMessage>,
        gop_clear: bool,
        receive_audio: bool,
        receive_video: bool,
    ) -> bool {
        let mut status_v = status.lock().await;

        match status_v.channels.get_mut(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let mut channel_status = channel_mu.lock().await;

                let player_status = RtmpPlayerStatus {
                    provided_key: key.to_string(),
                    message_sender: message_sender.clone(),
                    gop_clear: gop_clear,
                    paused: false,
                    idle: !channel_status.publishing,
                    receive_audio,
                    receive_video,
                };

                channel_status.players.insert(session_id, player_status);

                if !channel_status.publishing {
                    // Not publishing yet, stay idle until a publisher appears
                    return true;
                }

                if let Some(channel_key) = &channel_status.key {
                    if !string_compare_constant_time(&channel_key, key) {
                        // If the key is invalid, remove the player
                        channel_status.players.remove(&session_id);
                        _ = message_sender.send(RtmpSessionMessage::InvalidKey).await;
                        return false;
                    }
                }

                let publish_status = match &channel_status.publish_status {
                    Some(s) => s,
                    None => {
                        return true;
                    }
                };

                // Send the start message to the new player

                let player_start_msg = RtmpSessionPublishStreamStatus::get_play_start_message(
                    &publish_status,
                    gop_clear,
                )
                .await;

                _ = message_sender.send(player_start_msg).await;

                true
            }
            None => {
                let mut new_channel_status = RtmpChannelStatus::new();

                let player_status = RtmpPlayerStatus {
                    provided_key: key.to_string(),
                    message_sender: message_sender.clone(),
                    gop_clear: gop_clear,
                    paused: false,
                    idle: true,
                    receive_audio,
                    receive_video,
                };

                new_channel_status.players.insert(session_id, player_status);

                let channel_mu = Arc::new(Mutex::new(new_channel_status));

                status_v.channels.insert(channel.to_string(), channel_mu);

                // Since this channel is brand new, no publishing, so the player remains idle

                true
            }
        }
    }

    /// Removes a player from a channel
    pub async fn remove_player(status: &Mutex<RtmpServerStatus>, channel: &str, player_id: u64) {
        let mut status_v = status.lock().await;

        match status_v.channels.get_mut(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let mut channel_status = channel_mu.lock().await;

                channel_status.players.remove(&player_id);
            }
            None => {} // Nothing to do
        }
    }

    /// Send a packet to channel players
    pub async fn send_packet_to_channel(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        publisher_id: u64,
        packet: Arc<RtmpPacket>,
        config: &RtmpServerConfiguration,
    ) {
        let mut status_v = status.lock().await;

        match status_v.channels.get_mut(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let channel_status = channel_mu.lock().await;

                if let Some(pid) = channel_status.publisher_id {
                    if pid == publisher_id {
                        return;
                    }
                }

                let publish_status = match &channel_status.publish_status {
                    Some(s) => s,
                    None => {
                        return;
                    }
                };
                RtmpSessionPublishStreamStatus::push_new_packet(
                    publish_status,
                    packet.clone(),
                    config.gop_cache_size,
                )
                .await;

                // Send packet to players

                for (_, player) in &channel_status.players {
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
            None => {}
        }
    }

    /// Send a packet to channel players
    pub async fn set_channel_metadata(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        publisher_id: u64,
        metadata: Arc<Vec<u8>>,
    ) {
        let mut status_v = status.lock().await;

        match status_v.channels.get_mut(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let channel_status = channel_mu.lock().await;

                if let Some(pid) = channel_status.publisher_id {
                    if pid == publisher_id {
                        return;
                    }
                }

                let publish_status = match &channel_status.publish_status {
                    Some(s) => s,
                    None => {
                        return;
                    }
                };

                RtmpSessionPublishStreamStatus::set_metadata(publish_status, metadata.clone()).await;

                // Send metadata to players

                for (_, player) in &channel_status.players {
                    _ = player
                        .message_sender
                        .send(RtmpSessionMessage::PlayMetadata { metadata: metadata.clone() })
                        .await;
                }
            }
            None => {}
        }
    }

    /// Changes the receive_audio option for a player
    pub async fn player_set_receive_audio(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        player_id: u64,
        receive_audio: bool,
    ) {
        let mut status_v = status.lock().await;

        match status_v.channels.get_mut(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let mut channel_status = channel_mu.lock().await;

                match channel_status.players.get_mut(&player_id) {
                    Some(player_status) => {
                        player_status.receive_audio = receive_audio;
                    }
                    None => {}
                }
            }
            None => {} // Nothing to do
        }
    }

    /// Changes the receive_video option for a player
    pub async fn player_set_receive_video(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        player_id: u64,
        receive_video: bool,
    ) {
        let mut status_v = status.lock().await;

        match status_v.channels.get_mut(channel) {
            Some(c) => {
                let channel_mu = c.clone();
                drop(status_v);

                let mut channel_status = channel_mu.lock().await;

                match channel_status.players.get_mut(&player_id) {
                    Some(player_status) => {
                        player_status.receive_video = receive_video;
                    }
                    None => {}
                }
            }
            None => {} // Nothing to do
        }
    }
}
