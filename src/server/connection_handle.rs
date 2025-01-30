// Connection handling logic

use std::{net::IpAddr, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{log::Logger, session::{handle_rtmp_session, RtmpSessionPublishStreamStatus, RtmpSessionStatus}};

use super::{RtmpServerConfiguration, RtmpServerStatus, SessionIdGenerator};

/// Handles incoming connection (after accepting it)
/// read_stream - IO stream to read bytes
/// write_stream - IO stream to write bytes
/// ip - Client IP address
/// config - RTMP configuration
/// server_status - Server status
/// session_id_generator - Generator of IDs for the session
/// logger - Server logger
pub async fn handle_connection<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    read_stream: TR,
    write_stream: Arc<Mutex<TW>>,
    ip: IpAddr,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    session_id_generator: Arc<Mutex<SessionIdGenerator>>,
    logger: Arc<Logger>,
) {
    // Generate an unique ID for the session
    let mut session_id_generator_v = session_id_generator.as_ref().lock().await;
    let session_id = (*session_id_generator_v).generate_id();
    drop(session_id_generator_v);

    // Create a logger for the session
    let session_logger = Arc::new(
        logger
            .as_ref()
            .make_child_logger(&format!("[#{}] ", session_id)),
    );

    // Create status for the session
    let session_status = Arc::new(Mutex::new(RtmpSessionStatus::new()));
    let publish_status = Arc::new(Mutex::new(RtmpSessionPublishStreamStatus::new()));

    // Log request
    if config.log_requests {
        session_logger.log_info(&format!("Connection accepted from {}", ip.to_string()));
    }

    // Handle session
    handle_rtmp_session(
        session_id,
        read_stream,
        write_stream,
        config,
        server_status,
        session_status,
        publish_status,
        logger,
    )
    .await;
}
