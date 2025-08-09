// Delete stream command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    log_debug,
    rtmp::RtmpPacket,
    server::RtmpServerContext,
    session::{delete_stream::rtmp_delete_stream, SessionReadThreadContext},
};

/// Handles RTMP command: CLOSE STREAM
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `packet` - The packet containing the command
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_close_stream<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    packet: &RtmpPacket,
) -> bool {
    let stream_id = packet.header.stream_id;

    if stream_id == 0 {
        log_debug!(logger, "Command error: streamId cannot be 0");
    }

    rtmp_delete_stream(
        logger,
        server_context,
        session_context,
        write_stream,
        stream_id,
    )
    .await
}
