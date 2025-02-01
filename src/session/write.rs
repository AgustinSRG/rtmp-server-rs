use std::io::Error;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::rtmp::rtmp_make_status_message;

/// Write bytes to the session write stream
pub async fn session_write_bytes<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    write_stream: &Mutex<TW>,
    bytes: &[u8],
) -> Result<(), Error> {
    let mut write_stream_v = write_stream.lock().await;
    
    (*write_stream_v).write_all(bytes).await
}


/// Sends status message to client
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
