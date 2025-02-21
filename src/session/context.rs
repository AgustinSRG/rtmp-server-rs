// Context types to group parameters

use std::{net::IpAddr, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use super::{
    RtmpSessionMessage, RtmpSessionPlayStatus, RtmpSessionPublishStreamStatus,
    RtmpSessionReadStatus, RtmpSessionStatus,
};

/// Session context
#[derive(Clone)]
pub struct SessionContext {
    /// Session ID
    pub id: u64,

    /// Client IP address
    pub ip: IpAddr,

    /// Session status
    pub status: Arc<Mutex<RtmpSessionStatus>>,

    /// Publishing status
    pub publish_status: Arc<Mutex<RtmpSessionPublishStreamStatus>>,
}

impl SessionContext {
    /// Sets the session as killed
    pub async fn set_killed(&self) {
        let mut status = self.status.lock().await;

        status.killed = true;
    }

    /// Checks the play status of a session
    ///
    /// # Return value
    ///
    /// Returns the current player status of the session
    pub async fn play_status(&self) -> RtmpSessionPlayStatus {
        let status = self.status.lock().await;
        status.play_status.clone()
    }

    /// Checks the play status of a session
    ///
    /// # Return value
    ///
    /// Returns a tuple with 2 values:
    ///  1. True if the session is a player, false otherwise
    ///  2. If the session is a player, the ID of the internal RTMP stream used to play
    pub async fn play_stream_id(&self) -> (bool, u32) {
        let status = self.status.lock().await;
        (
            status.play_status.is_player,
            status.play_status.play_stream_id,
        )
    }

    /// Sets the playing status to false
    pub async fn stop_playing(&self) {
        let mut status_v = self.status.lock().await;
        status_v.play_status.is_player = false;
    }
}

/// Session context
pub struct SessionReadThreadContext {
    /// Session ID
    pub id: u64,

    /// Client IP address
    pub ip: IpAddr,

    /// Session status
    pub status: Arc<Mutex<RtmpSessionStatus>>,

    /// Publishing status
    pub publish_status: Arc<Mutex<RtmpSessionPublishStreamStatus>>,

    /// Sender for session messages
    pub session_msg_sender: Sender<RtmpSessionMessage>,

    /// Read status
    pub read_status: RtmpSessionReadStatus,
}

impl SessionReadThreadContext {
    /// Gets the current channel of the session
    pub async fn channel(&self) -> Option<String> {
        let status = self.status.lock().await;
        status.channel.clone()
    }

    /// Checks if the session is a publisher
    pub async fn is_publisher(&self) -> bool {
        let status = self.status.lock().await;

        status.is_publisher
    }

    /// Checks if the session is a player
    pub async fn is_player(&self) -> bool {
        let status = self.status.lock().await;

        status.play_status.is_player
    }

    /// Checks if the session is killed
    pub async fn is_killed(&self) -> bool {
        let status = self.status.lock().await;

        status.killed
    }

    /// Updates session status for publishing
    ///
    /// # Arguments
    ///
    /// * `publish_stream_id` - ID of the internal RTMP stream used for publishing
    pub async fn set_publisher(&self, publish_stream_id: u32) {
        let mut status = self.status.lock().await;

        status.is_publisher = true;
        status.publish_stream_id = publish_stream_id;
    }

    /// Updates session status for playing
    ///
    /// # Arguments
    ///
    /// * `receive_gop` - True for the player to receive packets from the GOP cache, false to receive only live packets
    /// * `play_stream_id` - ID of the internal RTMP stream used for playing
    ///
    /// # Return value
    ///
    /// Returns a tuple with 2 values:
    ///  1. The receive_audio setting (True to receive audio packets, false to ignore them)
    ///  2. The receive_video setting (True to receive video packets, false to ignore them)
    pub async fn set_player(&self, receive_gop: bool, play_stream_id: u32) -> (bool, bool) {
        let mut status = self.status.lock().await;

        status.play_status.is_player = true;
        status.play_status.receive_gop = receive_gop;
        status.publish_stream_id = play_stream_id;

        (
            status.play_status.receive_audio,
            status.play_status.receive_video,
        )
    }

    /// Sets the clock value for the publish status
    ///
    /// # Arguments
    ///
    /// * `clock_val` - Value of the clock to set
    pub async fn set_clock(&self, clock_val: i64) {
        let mut status = self.publish_status.lock().await;

        status.clock = clock_val;
    }
}
