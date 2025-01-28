// Logic to handle RTMP sessions

use std::sync::Arc;

use tokio::{io::{AsyncRead, AsyncWrite}, sync::Mutex};

use crate::{log::Logger, server::{RtmpServerConfiguration, RtmpServerStatus}};

use super::RtmpSessionStatus;

/// Handles RTMP session
/// id - Session ID
/// connection - IO stream to read and write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
pub async fn handle_rtmp_session<T: AsyncRead + AsyncWrite>(
    id: u64,
    connection: T,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    session_status: Arc<Mutex<RtmpSessionStatus>>,
    logger: Arc<Logger>,
) {
    // Handshake
}
