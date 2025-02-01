// Server status model

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use crate::{
    callback::make_stop_callback,
    control::ControlKeyValidationRequest,
    log::Logger,
    rtmp::{RtmpPacket, RTMP_TYPE_AUDIO, RTMP_TYPE_VIDEO},
    session::{RtmpSessionMessage, RtmpSessionPublishStreamStatus, RtmpSessionReadStatus},
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

                channel_status.publishing
            }
            None => false,
        }
    }

    /// Sets a publisher for a channel
    /// Return true if success, or false if another session was publishing on the channel
    #[allow(clippy::too_many_arguments)]
    pub async fn set_publisher(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        key: &str,
        stream_id: &str,
        session_id: u64,
        publish_status: Arc<Mutex<RtmpSessionPublishStreamStatus>>,
        message_sender: Sender<RtmpSessionMessage>,
        read_status: &mut RtmpSessionReadStatus,
    ) -> bool {
        let channel_status_ref: Arc<Mutex<RtmpChannelStatus>>;

        let mut status_v = status.lock().await;

        match status_v.channels.get(channel) {
            Some(channel_mu) => {
                let channel_mu_clone = channel_mu.clone();
                channel_status_ref = channel_mu.clone();
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

                channel_status_ref = channel_mu.clone();

                status_v.channels.insert(channel.to_string(), channel_mu);

                drop(status_v)
            }
        };

        read_status.channel_status = Some(channel_status_ref);

        true
    }

    /// Removes a player from a channel
    pub async fn remove_publisher(
        logger: &Logger,
        config: &RtmpServerConfiguration,
        status: &Mutex<RtmpServerStatus>,
        control_key_validator_sender: &mut Option<Sender<ControlKeyValidationRequest>>,
        channel: &str,
        publisher_id: u64,
    ) {
        let status_v = status.lock().await;

        if let Some(c) = status_v.channels.get(channel) {
            let channel_mu = c.clone();
            drop(status_v);

            let mut channel_status = channel_mu.lock().await;

            if !channel_status.publishing {
                return;
            }

            if let Some(pid) = channel_status.publisher_id {
                if pid != publisher_id {
                    return;
                }
            }

            // Unpublish

            let unpublished_stream_key = match &channel_status.key {
                Some(k) => k.clone(),
                None => "".to_string(),
            };

            let unpublished_stream_id = match &channel_status.stream_id {
                Some(i) => i.clone(),
                None => "".to_string(),
            };

            channel_status.publishing = false;
            channel_status.publisher_id = None;
            channel_status.publish_status = None;
            channel_status.publisher_message_sender = None;
            channel_status.key = None;
            channel_status.stream_id = None;

            // Notify players

            for player in channel_status.players.values_mut() {
                player.idle = true;
                _ = player
                    .message_sender
                    .send(RtmpSessionMessage::PlayStop)
                    .await;
            }

            drop(channel_status);

            // Send callback

            match control_key_validator_sender {
                Some(sender) => {
                    // Notify control server
                    _ = sender
                        .send(ControlKeyValidationRequest::PublishEnd {
                            channel: channel.to_string(),
                            stream_id: unpublished_stream_id,
                        })
                        .await;
                }
                None => {
                    // Callback
                    make_stop_callback(
                        logger,
                        &config.callback,
                        channel,
                        &unpublished_stream_key,
                        &unpublished_stream_id,
                    )
                    .await;
                }
            }
        }
    }

    /// Removes and kills a publisher
    pub async fn kill_publisher(
        logger: &Logger,
        config: &RtmpServerConfiguration,
        status: &Mutex<RtmpServerStatus>,
        control_key_validator_sender: &mut Option<Sender<ControlKeyValidationRequest>>,
        channel: &str,
        stream_id: Option<&str>,
    ) {
        let status_v = status.lock().await;

        if let Some(c) = status_v.channels.get(channel) {
                    let channel_mu = c.clone();
                    drop(status_v);

                    let mut channel_status = channel_mu.lock().await;

                    if !channel_status.publishing {
                        return;
                    }

                    if let Some(sid) = stream_id {
                        match &channel_status.stream_id {
                            Some(current_stream_id) => {
                                if *current_stream_id != sid {
                                    return; // Not the stream id we want to kill
                                }
                            },
                            None => {
                                return;
                            },
                        }
                    }
        
                    // Kill the publisher
        
                    if let Some(pub_sender) = &channel_status.publisher_message_sender {
                        _ = pub_sender.send(RtmpSessionMessage::Kill).await;
                    }
        
                    // Unpublish

                    let unpublished_stream_key = match &channel_status.key {
                        Some(k) => k.clone(),
                        None => "".to_string(),
                    };

                    let unpublished_stream_id = match &channel_status.stream_id {
                        Some(i) => i.clone(),
                        None => "".to_string(),
                    };
        
                    channel_status.publishing = false;
                    channel_status.publisher_id = None;
                    channel_status.publish_status = None;
                    channel_status.publisher_message_sender = None;
                    channel_status.key = None;
                    channel_status.stream_id = None;
        
                    // Notify players
        
                    for player in channel_status.players.values_mut() {
                        player.idle = true;
                        _ = player
                            .message_sender
                            .send(RtmpSessionMessage::PlayStop)
                            .await;
                    }
        
                    drop(channel_status);

                    // Send callback

                    match control_key_validator_sender {
                        Some(sender) => {
                            // Notify control server
                            _ = sender
                                .send(ControlKeyValidationRequest::PublishEnd {
                                    channel: channel.to_string(),
                                    stream_id: unpublished_stream_id,
                                })
                                .await;
                        }
                        None => {
                            // Callback
                            make_stop_callback(
                                logger,
                                &config.callback,
                                channel,
                                &unpublished_stream_key,
                                &unpublished_stream_id,
                            )
                            .await;
                        }
                    }
                }
    }

    /// Removes all the publishers and kills them
    pub async fn remove_all_publishers(
        status: &Mutex<RtmpServerStatus>,
    ) {
        let mut status_v = status.lock().await;

        let mut channels_to_delete: Vec<String> = Vec::new();

        for (channel, c) in &mut status_v.channels {
            let mut channel_status = c.lock().await;

            if !channel_status.publishing {
                continue;
            }

            // Kill the publisher

            if let Some(pub_sender) = &channel_status.publisher_message_sender {
                _ = pub_sender.send(RtmpSessionMessage::Kill).await;
            }

            // Unpublish

            channel_status.publishing = false;
            channel_status.publisher_id = None;
            channel_status.publish_status = None;
            channel_status.publisher_message_sender = None;
            channel_status.key = None;
            channel_status.stream_id = None;

            // Notify players

            for player in channel_status.players.values_mut() {
                player.idle = true;
                _ = player
                    .message_sender
                    .send(RtmpSessionMessage::PlayStop)
                    .await;
            }

            // Check if it can be deleted

            if channel_status.players.is_empty() {
                channels_to_delete.push(channel.clone());
            }
        }

        // Remove empty channels

        for channel in channels_to_delete {
            status_v.channels.remove(&channel);
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
    #[allow(clippy::too_many_arguments)]
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
                    gop_clear,
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
                    if !string_compare_constant_time(channel_key, key) {
                        // If the key is invalid, remove the player
                        channel_status.players.remove(&session_id);
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
                    publish_status,
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
                    gop_clear,
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

        if let Some(c) = status_v.channels.get_mut(channel) {
            let channel_mu = c.clone();
            drop(status_v);

            let mut channel_status = channel_mu.lock().await;

            channel_status.players.remove(&player_id);
        }
    }

    /// Send a packet to channel players
    pub async fn send_packet_to_channel(
        channel_mu: &Mutex<RtmpChannelStatus>,
        publisher_id: u64,
        packet: Arc<RtmpPacket>,
        skip_cache: bool,
        config: &RtmpServerConfiguration,
    ) {
        let channel_status = channel_mu.lock().await;

        if !channel_status.publishing {
            return;
        }

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

        if !skip_cache {
            RtmpSessionPublishStreamStatus::push_new_packet(
                publish_status,
                packet.clone(),
                config.gop_cache_size,
            )
            .await;
        }

        // Send packet to players

        for player in channel_status.players.values() {
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

    /// Send a packet to channel players
    pub async fn set_channel_metadata(
        status: &Mutex<RtmpServerStatus>,
        channel: &str,
        publisher_id: u64,
        metadata: Arc<Vec<u8>>,
    ) {
        let mut status_v = status.lock().await;

        if let Some(c) = status_v.channels.get_mut(channel) {
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

            RtmpSessionPublishStreamStatus::set_metadata(publish_status, metadata.clone())
                .await;

            // Send metadata to players

            for player in channel_status.players.values() {
                _ = player
                    .message_sender
                    .send(RtmpSessionMessage::PlayMetadata {
                        metadata: metadata.clone(),
                    })
                    .await;
            }
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

        if let Some(c) = status_v.channels.get_mut(channel) {
            let channel_mu = c.clone();
            drop(status_v);

            let mut channel_status = channel_mu.lock().await;

            if let Some(player_status) = channel_status.players.get_mut(&player_id) {
                player_status.receive_audio = receive_audio;
            }
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

        if let Some(c) = status_v.channels.get_mut(channel) {
            let channel_mu = c.clone();
            drop(status_v);

            let mut channel_status = channel_mu.lock().await;

            if let Some(player_status) = channel_status.players.get_mut(&player_id) {
                player_status.receive_video = receive_video;
            }
        }
    }

    /// Pauses a player
    pub async fn player_pause(status: &Mutex<RtmpServerStatus>, channel: &str, player_id: u64) {
        let mut status_v = status.lock().await;

        if let Some(c) = status_v.channels.get_mut(channel) {
            let channel_mu = c.clone();
            drop(status_v);

            let mut channel_status = channel_mu.lock().await;

            if let Some(player_status) = channel_status.players.get_mut(&player_id) {
                if player_status.paused {
                    return; // Already paused
                }

                player_status.paused = true;
                _ = player_status
                    .message_sender
                    .send(RtmpSessionMessage::Pause)
                    .await;
            }
        }
    }

    /// Resumes a player
    pub async fn player_resume(status: &Mutex<RtmpServerStatus>, channel: &str, player_id: u64) {
        let mut status_v = status.lock().await;

        if let Some(c) = status_v.channels.get_mut(channel) {
            let channel_mu = c.clone();
            drop(status_v);

            let mut channel_status = channel_mu.lock().await;

            let publishing = channel_status.publishing;
            let publish_status = channel_status.publish_status.clone();

            if let Some(player_status) = channel_status.players.get_mut(&player_id) {
                if !player_status.paused {
                    return; // Not paused
                }

                player_status.paused = false;

                if publishing {
                    if let Some(publish_status) = &publish_status {
                        let player_resume_message =
                            RtmpSessionPublishStreamStatus::get_player_resume_message(
                                publish_status,
                            )
                            .await;

                        _ = player_status.message_sender.send(player_resume_message);
                    } else {
                        _ = player_status
                            .message_sender
                            .send(RtmpSessionMessage::ResumeIdle)
                            .await;
                    }
                } else {
                    _ = player_status
                        .message_sender
                        .send(RtmpSessionMessage::ResumeIdle)
                        .await;
                }
            }
        }
    }
}
