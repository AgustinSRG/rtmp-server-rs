use std::io::Error;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

/// Write bytes to the session write stream
pub async fn session_write_bytes<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    write_stream: &Mutex<TW>,
    bytes: &[u8],
) -> Result<(), Error> {
    let mut write_stream_v = write_stream.lock().await;
    let res = (*write_stream_v).write_all(bytes).await;
    res
}
