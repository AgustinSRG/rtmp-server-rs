// Message read logic

use std::sync::Arc;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Receiver, Mutex},
};

use crate::{
    log::Logger,
    log_debug, log_trace,
    rtmp::{
        rtmp_make_audio_codec_header_message, rtmp_make_metadata_message,
        rtmp_make_sample_access_message, rtmp_make_stream_status_message,
        rtmp_make_video_codec_header_message, RTMP_TYPE_AUDIO, RTMP_TYPE_VIDEO, STREAM_BEGIN,
        STREAM_EOF,
    },
    server::RtmpServerContext,
};

use super::{
    do_session_cleanup, send_status_message, session_write_bytes, RtmpSessionMessage,
    SessionContext,
};

/// Handles session message
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `msg` - The message
pub async fn handle_session_message<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: &Logger,
    server_context: &RtmpServerContext,
    session_context: &SessionContext,
    write_stream: &Mutex<TW>,
    msg: RtmpSessionMessage,
) -> bool {
    let server_config = &server_context.config;
    match msg {
        RtmpSessionMessage::PlayStart {
            metadata,
            audio_codec,
            aac_sequence_header,
            video_codec,
            avc_sequence_header,
            gop_cache,
        } => {
            log_debug!(logger, "RtmpSessionMessage::PlayStart");

            // Get play status
            let play_status = session_context.play_status().await;

            if !play_status.is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes =
                rtmp_make_stream_status_message(STREAM_BEGIN, play_status.play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not send stream status: {}", e)
                );

                return true;
            }

            log_debug!(logger, "RtmpSessionMessage::PlayStart - Sent stream status");

            // Send status messages indicating play

            if let Err(e) = send_status_message(
                write_stream,
                play_status.play_stream_id,
                "status",
                "NetStream.Play.Reset",
                Some("Playing and resetting stream."),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }

            if let Err(e) = send_status_message(
                write_stream,
                play_status.play_stream_id,
                "status",
                "NetStream.Play.Start",
                Some("Started playing stream."),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }

            log_debug!(
                logger,
                "RtmpSessionMessage::PlayStart - Sent status messages"
            );

            // Send sample access message

            let sample_access_bytes = rtmp_make_sample_access_message(0, server_config.chunk_size);

            if let Err(e) = session_write_bytes(write_stream, &sample_access_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not send sample access: {}", e)
                );

                return true;
            }

            // Send metadata

            if !metadata.is_empty() {
                let metadata_bytes = rtmp_make_metadata_message(
                    play_status.play_stream_id,
                    &metadata,
                    0,
                    server_config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &metadata_bytes).await {
                    log_debug!(
                        logger,
                        format!("Send error: Could not send metadata bytes: {}", e)
                    );

                    return true;
                }

                log_debug!(
                    logger,
                    "RtmpSessionMessage::PlayStart - Sent metadata message"
                );
            }

            // Send audio codec header

            if audio_codec == 10 || audio_codec == 13 {
                let audio_codec_header = rtmp_make_audio_codec_header_message(
                    play_status.play_stream_id,
                    &aac_sequence_header,
                    0,
                    server_config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &audio_codec_header).await {
                    log_debug!(
                        logger,
                        format!("Send error: Could not send audio codec header: {}", e)
                    );

                    return true;
                }

                log_debug!(logger, "Sent audio codec header");
            }

            // Send video codec header

            if video_codec == 7 || video_codec == 12 {
                let video_codec_header = rtmp_make_video_codec_header_message(
                    play_status.play_stream_id,
                    &avc_sequence_header,
                    0,
                    server_config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &video_codec_header).await {
                    log_debug!(
                        logger,
                        format!("Send error: Could not send video codec header: {}", e)
                    );

                    return true;
                }

                log_debug!(logger, "Sent video codec header");
            }

            // Send GOP cache

            if play_status.receive_gop {
                for packet in gop_cache {
                    if packet.header.packet_type == RTMP_TYPE_AUDIO && !play_status.receive_audio {
                        continue;
                    }

                    if packet.header.packet_type == RTMP_TYPE_VIDEO && !play_status.receive_video {
                        continue;
                    }

                    let packet_bytes = packet.create_chunks_for_stream(
                        play_status.play_stream_id,
                        server_config.chunk_size,
                    );

                    if let Err(e) = session_write_bytes(write_stream, &packet_bytes).await {
                        log_debug!(
                            logger,
                            format!("Send error: Could not send GOP cached packet: {}", e)
                        );

                        return true;
                    }

                    log_debug!(
                        logger,
                        format!(
                            "RtmpSessionMessage::PlayStart - Sent GOP packet: {} bytes",
                            packet.payload.len()
                        )
                    );
                }
            }

            // Log

            log_debug!(logger, "Changed play status: PLAYING");
        }
        RtmpSessionMessage::InvalidKey => {
            log_debug!(logger, "RtmpSessionMessage::InvalidKey");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            // Set playing status to false
            session_context.stop_playing().await;

            // Send status message

            log_debug!(logger, "Invalid play stream key provided");

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "error",
                "NetStream.Publish.BadName",
                Some("Invalid stream key provided"),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }
        }
        RtmpSessionMessage::PlayMetadata { metadata } => {
            log_debug!(logger, "RtmpSessionMessage::PlayMetadata");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            if metadata.is_empty() {
                return true;
            }

            // Make metadata message

            let metadata_bytes =
                rtmp_make_metadata_message(play_stream_id, &metadata, 0, server_config.chunk_size);

            // Send metadata

            if let Err(e) = session_write_bytes(write_stream, &metadata_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not channel metadata: {}", e)
                );
                return true;
            }
        }
        RtmpSessionMessage::PlayPacket { packet } => {
            log_trace!(logger, "RtmpSessionMessage::PlayPacket");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            let packet_bytes =
                packet.create_chunks_for_stream(play_stream_id, server_config.chunk_size);

            if let Err(e) = session_write_bytes(write_stream, &packet_bytes).await {
                log_debug!(logger, format!("Send error: Could not send packet: {}", e));

                return true;
            }
        }
        RtmpSessionMessage::PlayStop => {
            log_debug!(logger, "RtmpSessionMessage::PlayStop");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Play.UnpublishNotify",
                Some("stream is now unpublished."),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_EOF, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not send stream status: {}", e)
                );

                return true;
            }

            // Log

            log_debug!(logger, "Changed play status: IDLE");
        }
        RtmpSessionMessage::Pause => {
            log_debug!(logger, "RtmpSessionMessage::Pause");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_EOF, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not send stream status: {}", e)
                );
                return true;
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Pause.Notify",
                Some("Paused live"),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }

            // Log

            log_debug!(logger, "Changed play status: PAUSED");
        }
        RtmpSessionMessage::Resume {
            audio_codec,
            aac_sequence_header,
            video_codec,
            avc_sequence_header,
        } => {
            log_debug!(logger, "RtmpSessionMessage::Resume");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_BEGIN, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not send stream status: {}", e)
                );
                return true;
            }

            // Send audio codec header

            if audio_codec == 10 || audio_codec == 13 {
                let audio_codec_header = rtmp_make_audio_codec_header_message(
                    play_stream_id,
                    &aac_sequence_header,
                    0,
                    server_config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &audio_codec_header).await {
                    log_debug!(
                        logger,
                        format!("Send error: Could not send audio codec header: {}", e)
                    );

                    return true;
                }

                log_debug!(logger, "Sent audio codec header");
            }

            // Send video codec header

            if video_codec == 7 || video_codec == 12 {
                let video_codec_header = rtmp_make_video_codec_header_message(
                    play_stream_id,
                    &avc_sequence_header,
                    0,
                    server_config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &video_codec_header).await {
                    log_debug!(
                        logger,
                        format!("Send error: Could not send video codec header: {}", e)
                    );

                    return true;
                }

                log_debug!(logger, "Sent video codec header");
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Unpause.Notify",
                Some("Unpaused live"),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }

            // Log

            log_debug!(logger, "Changed play status: PLAYING");
        }
        RtmpSessionMessage::ResumeIdle => {
            log_debug!(logger, "RtmpSessionMessage::ResumeIdle");

            // Get play status
            let (is_player, play_stream_id) = session_context.play_stream_id().await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_BEGIN, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                log_debug!(
                    logger,
                    format!("Send error: Could not send stream status: {}", e)
                );
                return true;
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Unpause.Notify",
                Some("Unpaused live"),
                server_config.chunk_size,
            )
            .await
            {
                log_debug!(
                    logger,
                    format!("Send error: Could not send status message: {}", e)
                );
            }

            // Log

            log_debug!(logger, "Changed play status: IDLE");
        }
        RtmpSessionMessage::Kill => {
            log_debug!(logger, "RtmpSessionMessage::Kill");

            session_context.set_killed().await;
        }
        RtmpSessionMessage::End => {
            log_debug!(logger, "RtmpSessionMessage::End");

            return false;
        }
    }

    true
}

/// Creates a task to read and handle session messages
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `write_stream` - The stream to write to the client
/// * `session_msg_receiver` - The receiver to read session messages from
pub fn spawn_task_to_read_session_messages<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: Arc<Logger>,
    mut server_context: RtmpServerContext,
    session_context: SessionContext,
    write_stream: Arc<Mutex<TW>>,
    mut session_msg_receiver: Receiver<RtmpSessionMessage>,
) {
    tokio::spawn(async move {
        let mut continue_loop = true;

        while continue_loop {
            let msg_opt = session_msg_receiver.recv().await;

            match msg_opt {
                Some(msg) => {
                    continue_loop = handle_session_message(
                        &logger,
                        &server_context,
                        &session_context,
                        &write_stream,
                        msg,
                    )
                    .await;
                }
                None => {
                    continue_loop = false;
                }
            }
        }

        // Cleanup

        log_debug!(logger, "Performing session cleanup...");

        do_session_cleanup(&logger, &mut server_context, &session_context).await;

        log_debug!(logger, "Draining message channel...");

        // Drain channel

        while session_msg_receiver.try_recv().is_ok() {} // Drain the channel to prevent other threads from blocking

        log_debug!(logger, "Completed session messages handling task");
    });
}
