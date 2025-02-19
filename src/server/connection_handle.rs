// Connection handling logic

use std::{net::IpAddr, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    session::{handle_rtmp_session, RtmpSessionPublishStreamStatus, RtmpSessionStatus, SessionContext},
};

use super::{RtmpServerContext, RtmpServerContextExtended};

/// Handles incoming connection (after accepting it)
///
/// # Arguments
///
/// * `logger` - The server logger
/// * `server_context` - The server context
/// * `read_stream` - The stream to read from the client
/// * `write_stream` - The stream to write to the client
/// * `ip` - The client IP address
pub async fn handle_connection<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: Arc<Logger>,
    server_context: RtmpServerContextExtended,
    read_stream: TR,
    write_stream: Arc<Mutex<TW>>,
    ip: IpAddr,
) {
    // Generate an unique ID for the session
    let mut session_id_generator_v = server_context.session_id_generator.as_ref().lock().await;
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
    if server_context.config.log_requests {
        session_logger.log_info(&format!("Connection accepted from {}", ip));
    }

    // Create session context
    let session_context = SessionContext{
        id: session_id,
        ip,
        status: session_status,
        publish_status,
    };

    // Handle session
    handle_rtmp_session(
        session_logger,
        RtmpServerContext{
            config: server_context.config,
            status: server_context.status,
            control_key_validator_sender: server_context.control_key_validator_sender,
        },
        session_context,
        read_stream,
        write_stream,
    )
    .await;
}
