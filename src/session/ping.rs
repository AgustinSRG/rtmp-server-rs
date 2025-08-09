// Logic to send pings to session

use std::{sync::Arc, time::Duration};

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Receiver, Mutex},
};

use crate::{
    log::Logger,
    log_debug,
    rtmp::{rtmp_make_ping_request, RTMP_PING_TIME},
    server::RtmpServerContext,
    session::session_write_bytes,
};

use super::SessionContext;

/// Creates a task to send ping requests to the client
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `cancel_pings_receiver` - A receiver to listen for a cancel signal
pub fn spawn_task_to_send_pings<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    logger: Arc<Logger>,
    server_context: RtmpServerContext,
    session_context: SessionContext,
    write_stream: Arc<Mutex<TW>>,
    mut cancel_pings_receiver: Receiver<()>,
) {
    tokio::spawn(async move {
        let mut finished = false;
        while !finished {
            // Wait
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(RTMP_PING_TIME)) => {}
                _ = cancel_pings_receiver.recv() => {
                    finished = true;
                    continue;
                }
            }

            // Check status
            let session_status_v = session_context.status.lock().await;

            if session_status_v.channel.is_none() {
                drop(session_status_v);
                continue; // Not connected, can't send ping yet
            }

            let connect_time = session_status_v.connect_time;

            drop(session_status_v);

            // Create ping

            let ping_bytes = rtmp_make_ping_request(connect_time, server_context.config.chunk_size);

            log_debug!(logger, "Sending ping request to client");

            // Send packet

            match session_write_bytes(&write_stream, &ping_bytes).await {
                Ok(_) => {
                    log_debug!(logger, "Sent ping request to client");
                }
                Err(e) => {
                    log_debug!(logger, format!("Could not send ping request: {}", e));

                    finished = true;
                }
            }
        }
    });
}
