// Invoke packet handling logic

use std::sync::Arc;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{RtmpCommand, RtmpPacket, RTMP_TYPE_FLEX_MESSAGE},
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{
    handle_rtmp_command_connect, handle_rtmp_command_create_stream, handle_rtmp_command_publish,
    handle_rtmp_command_receive_audio, handle_rtmp_command_receive_video, RtmpSessionMessage,
    RtmpSessionPublishStreamStatus, RtmpSessionReadStatus, RtmpSessionStatus,
};

/// Handles RTMP packet (INVOKE)
/// packet - The packet to handle
/// session_id - Session ID
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// session_msg_sender - Message sender for the session
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet_invoke<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    packet: &RtmpPacket,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    read_status: &mut RtmpSessionReadStatus,
    logger: &Logger,
) -> bool {
    let offset: usize = if packet.header.packet_type == RTMP_TYPE_FLEX_MESSAGE {
        1
    } else {
        0
    };

    if packet.header.length <= offset {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Packet error: Packet length too short");
        }

        return false;
    }

    if packet.header.length > packet.payload.len() {
        if config.log_requests {
            logger.log_error("Packet error: Payload does not match with packet length");
        }

        return false;
    }

    let cmd = match RtmpCommand::decode(&packet.payload[offset..packet.header.length]) {
        Ok(c) => c,
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Packet error: Could not decode RTMP command");
            }

            return false;
        }
    };

    if config.log_requests && logger.config.trace_enabled {
        logger.log_trace(&format!("DATA COMMAND: {}", cmd.to_debug_string()));
    }

    match cmd.cmd.as_str() {
        "connect" => {
            handle_rtmp_command_connect(&cmd, write_stream, config, session_status, logger).await
        }
        "createStream" => {
            handle_rtmp_command_create_stream(&cmd, write_stream, config, session_status, logger)
                .await
        }
        "publish" => {
            handle_rtmp_command_publish(
                packet,
                &cmd,
                session_id,
                write_stream,
                config,
                server_status,
                session_status,
                publish_status,
                session_msg_sender,
                read_status,
                logger,
            )
            .await
        }
        "play" => true,
        "pause" => true,
        "deleteStream" => true,
        "closeStream" => true,
        "receiveAudio" => {
            handle_rtmp_command_receive_audio(
                &cmd,
                session_id,
                config,
                server_status,
                session_status,
                logger,
            )
            .await
        }
        "receiveVideo" => {
            handle_rtmp_command_receive_video(
                &cmd,
                session_id,
                config,
                server_status,
                session_status,
                logger,
            )
            .await
        }
        _ => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Unrecognized command: {}", cmd.cmd));
            }

            true
        }
    }
}
