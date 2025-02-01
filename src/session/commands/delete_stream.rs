// Delete stream command

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    control::ControlKeyValidationRequest, log::Logger, rtmp::RtmpCommand, server::{RtmpServerConfiguration, RtmpServerStatus}, session::delete_stream::rtmp_delete_stream
};

use super::super::RtmpSessionStatus;

/// Handles RTMP command (deleteStream)
/// cmd - The command to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// control_key_validator_sender - Sender for key validation against the control server
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_command_delete_stream<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    cmd: &RtmpCommand,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    control_key_validator_sender: &mut Option<Sender<ControlKeyValidationRequest>>,
    logger: &Logger,
) -> bool {
    let stream_id = match cmd.get_argument("streamId") {
        Some(i) => i.get_integer() as u32,
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: streamId property not provided");
            }

            return true;
        }
    };

    if stream_id == 0 {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Command error: streamId cannot be 0");
        }
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
