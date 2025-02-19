// Context types to group parameters

use std::sync::Arc;

use tokio::sync::{mpsc::Sender, Mutex};

use crate::control::ControlKeyValidationRequest;

use super::{IpConnectionCounter, RtmpServerConfiguration, RtmpServerStatus, SessionIdGenerator};

/// RTMP server context
#[derive(Clone)]
pub struct RtmpServerContext {
    /// Server configuration
    pub config: Arc<RtmpServerConfiguration>,

    /// Server status
    pub status: Arc<Mutex<RtmpServerStatus>>,

    /// Sender for key validation against the control server
    pub control_key_validator_sender: Option<Sender<ControlKeyValidationRequest>>,
}

/// RTMP server context
#[derive(Clone)]
pub struct RtmpServerContextExtended {
    /// Server configuration
    pub config: Arc<RtmpServerConfiguration>,

    /// Server status
    pub status: Arc<Mutex<RtmpServerStatus>>,

    /// Sender for key validation against the control server
    pub control_key_validator_sender: Option<Sender<ControlKeyValidationRequest>>,

    /// IP counter
    pub ip_counter: Arc<Mutex<IpConnectionCounter>>,

    /// Session ID generator
    pub session_id_generator: Arc<Mutex<SessionIdGenerator>>,
}
