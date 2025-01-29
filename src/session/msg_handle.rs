// Message read logic

use std::sync::Arc;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Receiver, Mutex},
};

use crate::{
    log::Logger,
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{do_session_cleanup, RtmpSessionMessage, RtmpSessionStatus};

/// Handles session message
/// msg - Session message to handle
/// session_id - Session ID
/// write_stream - IO stream to read and write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
pub async fn handle_session_message<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    msg: RtmpSessionMessage,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    match msg {
        RtmpSessionMessage::PlayStart {
            metadata,
            audio_codec,
            aac_sequence_header,
            video_codec,
            avc_sequence_header,
            gop_cache,
        } => {}
        RtmpSessionMessage::PlayMetadata { metadata } => {}
        RtmpSessionMessage::PlayPacket { packet } => {}
        RtmpSessionMessage::PlayStop => {}
        RtmpSessionMessage::InvalidKey => {}
        RtmpSessionMessage::End => {
            return false;
        }
    }

    true
}

/// Creates a task to read and handle session messages
/// msg - Session message to handle
/// session_id - Session ID
/// write_stream - IO stream to read and write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
pub fn spawn_task_to_read_session_messages<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    session_id: u64,
    write_stream: Arc<Mutex<TW>>,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    session_status: Arc<Mutex<RtmpSessionStatus>>,
    mut session_msg_receiver: Receiver<RtmpSessionMessage>,
    logger: Arc<Logger>,
) {
    tokio::spawn(async move {
        let mut continue_loop = true;

        while continue_loop {
            let msg_opt = session_msg_receiver.recv().await;

            match msg_opt {
                Some(msg) => {
                    continue_loop = handle_session_message(
                        msg,
                        session_id,
                        &write_stream,
                        &config,
                        &server_status,
                        &session_status,
                        &logger,
                    )
                    .await;
                }
                None => {
                    continue_loop = false;
                }
            }
        }

        // Cleanup

        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Performing session cleanup...");
        }

        do_session_cleanup(
            session_id,
            &config,
            &server_status,
            &session_status,
            &logger,
        )
        .await;

        // Drain channel

        while let Ok(_) = session_msg_receiver.try_recv() {} // Drain the channel to prevent other threads from blocking
    });
}
