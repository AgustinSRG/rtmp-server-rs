use std::io::Error;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::rtmp::rtmp_make_status_message;

/// Writes bytes to the session write stream
///
/// # Arguments
///
/// * `write_stream` - The stream to write to the client
/// * `bytes` - The bytes to write
pub async fn session_write_bytes<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    write_stream: &Mutex<TW>,
    bytes: &[u8],
) -> Result<(), Error> {
    let mut write_stream_v = write_stream.lock().await;

    (*write_stream_v).write_all(bytes).await
}

/// Sends RTMP status message to the client
///
/// # Arguments
///
/// * `write_stream` - The stream to write to the client
/// * `stream_id` - Stream ID subject of the status message
/// * `level` - Status message level
/// * `code` - Status code
/// * `description` - Status description
/// * `out_chunk_size` - Chunk size, in order to generate the RTMP packet chunks
pub async fn send_status_message<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    write_stream: &Mutex<TW>,
    stream_id: u32,
    level: &str,
    code: &str,
    description: Option<&str>,
    out_chunk_size: usize,
) -> Result<(), Error> {
    let msg_bytes = rtmp_make_status_message(stream_id, level, code, description, out_chunk_size);
    session_write_bytes(write_stream, &msg_bytes).await
}
