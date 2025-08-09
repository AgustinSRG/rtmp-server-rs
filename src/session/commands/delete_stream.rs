// Delete stream command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    log_debug,
    rtmp::RtmpCommand,
    server::RtmpServerContext,
    session::{delete_stream::rtmp_delete_stream, SessionReadThreadContext},
};

/// Handles RTMP command: DELETE STREAM
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
pub async fn handle_rtmp_command_delete_stream<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    cmd: &RtmpCommand,
) -> bool {
    let stream_id = match cmd.get_argument("streamId") {
        Some(i) => i.get_integer() as u32,
        None => {
            log_debug!(logger, "Command error: streamId property not provided");

            return true;
        }
    };

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
