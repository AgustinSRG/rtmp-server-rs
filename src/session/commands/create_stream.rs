// Create stream command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    rtmp::{rtmp_make_create_stream_response, RtmpCommand},
    server::RtmpServerContext,
    session::SessionReadThreadContext,
};

use super::super::session_write_bytes;

/// Handles RTMP command: CREATE STREAM
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `cmd` - The command
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_create_stream<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    cmd: &RtmpCommand,
) -> bool {
    // Load and validate parameters

    let trans_id = match cmd.get_argument("transId") {
        Some(t) => t.get_integer(),
        None => 0,
    };

    // Create stream

    let mut session_status_v = session_context.status.lock().await;
    session_status_v.streams = session_status_v.streams.wrapping_add(1);
    let stream_index = session_status_v.streams as u32;
    drop(session_status_v);

    // Respond

    let response_bytes =
        rtmp_make_create_stream_response(trans_id, stream_index, server_context.config.chunk_size);
    if let Err(e) = session_write_bytes(write_stream, &response_bytes).await {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send connect response: {}",
                e
            ));
        }
        return false;
    }

    // Done

    true
}
