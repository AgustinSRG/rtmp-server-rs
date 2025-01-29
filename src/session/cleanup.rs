// Session cleanup logic

use tokio::sync::Mutex;

use crate::{
    log::Logger,
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::RtmpSessionStatus;

/// Performs session cleanup
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
pub async fn do_session_cleanup(
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) {
    
}
