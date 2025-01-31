// Create stream command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    rtmp::{rtmp_make_create_stream_response, RtmpCommand},
    server::RtmpServerConfiguration,
};

use super::super::{session_write_bytes, RtmpSessionStatus};

/// Handles RTMP command (createStream)
/// cmd - The command to handle
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_create_stream<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    cmd: &RtmpCommand,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let trans_id = match cmd.get_argument("transId") {
        Some(t) => t.get_integer(),
        None => 0,
    };

    // Create stream

    let mut session_status_v = session_status.lock().await;
    session_status_v.streams = session_status_v.streams.wrapping_add(1);
    let stream_index = session_status_v.streams as u32;
    drop(session_status_v);

    // Respond

    let response_bytes =
        rtmp_make_create_stream_response(trans_id, stream_index, config.chunk_size);
    if let Err(e) = session_write_bytes(&write_stream, &response_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send connect response: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Done

    true
}
