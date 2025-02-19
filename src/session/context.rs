// Context types to group parameters

use std::{net::IpAddr, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use super::{
    RtmpSessionMessage, RtmpSessionPublishStreamStatus, RtmpSessionReadStatus, RtmpSessionStatus,
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
