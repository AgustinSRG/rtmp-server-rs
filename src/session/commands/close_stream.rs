// Delete stream command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    control::ControlKeyValidationRequest, log::Logger, rtmp::RtmpPacket, server::{RtmpServerConfiguration, RtmpServerStatus}, session::delete_stream::rtmp_delete_stream
};

use super::super::RtmpSessionStatus;

/// Handles RTMP command (deleteStream)
/// packet - The packet to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// control_key_validator_sender - Sender for key validation against the control server
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
#[allow(clippy::too_many_arguments)]
pub async fn handle_rtmp_command_close_stream<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    packet: &RtmpPacket,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    control_key_validator_sender: &mut Option<Sender<ControlKeyValidationRequest>>,
    logger: &Logger,
) -> bool {
    let stream_id = packet.header.stream_id;

    if stream_id == 0 && config.log_requests && logger.config.debug_enabled {
        logger.log_debug("Command error: streamId cannot be 0");
    }

    rtmp_delete_stream(
        stream_id,
        session_id,
        write_stream,
        config,
        server_status,
        session_status,
        control_key_validator_sender,
        logger,
    )
    .await
}
