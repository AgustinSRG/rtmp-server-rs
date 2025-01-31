// Callback events

use std::net::IpAddr;

/// Callback event
pub enum CallbackEvent {
    /// Start event to check the key
    Start { client_ip: IpAddr },
    /// Stop event
    Stop { stream_id: String },
}

impl CallbackEvent {
    /// Gets event
    pub fn get_event(&self) -> String {
        match self {
            CallbackEvent::Start { client_ip: _ } => "start".to_string(),
            CallbackEvent::Stop { stream_id: _ } => "stop".to_string(),
        }
    }

    /// Gets stream ID
    pub fn get_stream_id(&self) -> Option<String> {
        match self {
            CallbackEvent::Start { client_ip: _ } => None,
            CallbackEvent::Stop { stream_id } => Some(stream_id.clone()),
        }
    }
    /// Gets client IP
    pub fn get_client_ip(&self) -> Option<String> {
        match self {
            CallbackEvent::Start { client_ip } => Some(client_ip.to_string()),
            CallbackEvent::Stop { stream_id: _ } => None,
        }
    }
}
