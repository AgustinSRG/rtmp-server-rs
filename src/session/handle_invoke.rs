// Invoke packet handling logic

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    log_error,
    rtmp::{RtmpCommand, RtmpPacket, RTMP_TYPE_FLEX_MESSAGE},
    server::RtmpServerContext,
};

use super::{
    handle_rtmp_command_close_stream, handle_rtmp_command_connect,
    handle_rtmp_command_create_stream, handle_rtmp_command_delete_stream,
    handle_rtmp_command_pause, handle_rtmp_command_play, handle_rtmp_command_publish,
    handle_rtmp_command_receive_audio, handle_rtmp_command_receive_video, SessionReadThreadContext,
};

/// Handles INVOKE RTMP packet
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `packet` - The packet
///
/// # Return value
///
/// Returns true to continue receiving chunks. Returns false to end the session main loop.
pub async fn handle_rtmp_packet_invoke<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &mut RtmpServerContext,
    session_context: &mut SessionReadThreadContext,
    write_stream: &Mutex<TW>,
    packet: &RtmpPacket,
) -> bool {
    let offset: usize = if packet.header.packet_type == RTMP_TYPE_FLEX_MESSAGE {
        1
    } else {
        0
    };

    if packet.header.length <= offset {
        if server_context.config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Packet error: Packet length too short");
        }

        return false;
    }

    if packet.header.length > packet.payload.len() {
        log_error!(
            logger,
            "Packet error: Payload does not match with packet length"
        );

        return false;
    }

    let cmd = match RtmpCommand::decode(&packet.payload[offset..packet.header.length]) {
        Ok(c) => c,
        Err(_) => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Packet error: Could not decode RTMP command");
            }

            return false;
        }
    };

    if server_context.config.log_requests && logger.config.trace_enabled {
        logger.log_trace(&format!("COMMAND: {}", cmd.to_debug_string()));
    }

    match cmd.cmd.as_str() {
        "connect" => {
            handle_rtmp_command_connect(logger, server_context, session_context, write_stream, &cmd)
                .await
        }
        "createStream" => {
            handle_rtmp_command_create_stream(
                logger,
                server_context,
                session_context,
                write_stream,
                &cmd,
            )
            .await
        }
        "publish" => {
            handle_rtmp_command_publish(
                logger,
                server_context,
                session_context,
                write_stream,
                packet,
                &cmd,
            )
            .await
        }
        "play" => {
            handle_rtmp_command_play(
                logger,
                server_context,
                session_context,
                write_stream,
                packet,
                &cmd,
            )
            .await
        }
        "pause" => handle_rtmp_command_pause(logger, server_context, session_context, &cmd).await,
        "deleteStream" => {
            handle_rtmp_command_delete_stream(
                logger,
                server_context,
                session_context,
                write_stream,
                &cmd,
            )
            .await
        }
        "closeStream" => {
            handle_rtmp_command_close_stream(
                logger,
                server_context,
                session_context,
                write_stream,
                packet,
            )
            .await
        }
        "receiveAudio" => {
            handle_rtmp_command_receive_audio(logger, server_context, session_context, &cmd).await
        }
        "receiveVideo" => {
            handle_rtmp_command_receive_video(logger, server_context, session_context, &cmd).await
        }
        _ => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!("Unrecognized command: {}", cmd.cmd));
            }

            true
        }
    }
}
