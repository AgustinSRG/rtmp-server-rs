// Logic to send pings to session

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Receiver, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{rtmp_make_ping_request, RtmpPacket, RTMP_CHANNEL_PROTOCOL, RTMP_CHUNK_TYPE_0, RTMP_PING_TIME, RTMP_TYPE_EVENT},
    server::RtmpServerConfiguration,
    session::session_write_bytes,
};

use super::RtmpSessionStatus;

pub fn spawn_task_to_send_pings<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static>(
    write_stream: Arc<Mutex<TW>>,
    config: Arc<RtmpServerConfiguration>,
    session_status: Arc<Mutex<RtmpSessionStatus>>,
    mut cancel_pings_receiver: Receiver<()>,
    logger: Arc<Logger>,
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
            let session_status_v = session_status.lock().await;

            if session_status_v.channel.is_none() {
                drop(session_status_v);
                continue; // Not connected, can't send ping yet
            }

            let connect_time = session_status_v.connect_time;

            drop(session_status_v);

            // Create ping

            let ping_bytes = rtmp_make_ping_request(connect_time, config.chunk_size);

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Sending ping request to client");
            }

            // Send packet

            match session_write_bytes(&write_stream, &ping_bytes).await {
                Ok(_) => {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug("Sent ping request to client");
                    }
                }
                Err(e) => {
                    if config.log_requests && logger.config.debug_enabled {
                        logger
                            .log_debug(&format!("Could not send ping request: {}", e.to_string()));
                    }

                    finished = true;
                }
            }
        }
    });
}
