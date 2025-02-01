// Message read logic

use std::sync::Arc;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

use crate::{
    control::ControlKeyValidationRequest,
    log::Logger,
    rtmp::{
        rtmp_make_audio_codec_header_message, rtmp_make_metadata_message,
        rtmp_make_sample_access_message, rtmp_make_stream_status_message,
        rtmp_make_video_codec_header_message, RTMP_TYPE_AUDIO, RTMP_TYPE_VIDEO, STREAM_BEGIN,
        STREAM_EOF,
    },
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{
    do_session_cleanup, send_status_message, session_write_bytes, RtmpSessionMessage,
    RtmpSessionStatus,
};

/// Handles session message
/// msg - Session message to handle
/// session_id - Session ID
/// write_stream - IO stream to read and write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// logger - Session logger
pub async fn handle_session_message<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    msg: RtmpSessionMessage,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
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
        } => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::PlayStart");
            }

            // Get play status
            let (is_player, play_stream_id, receive_gop, receive_audio, receive_video) =
                RtmpSessionStatus::check_play_status(session_status).await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_BEGIN, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send stream status: {}", e));
                }
                return true;
            }

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::PlayStart - Sent stream status");
            }

            // Send status messages indicating play

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Play.Reset",
                Some("Playing and resetting stream."),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Play.Start",
                Some("Started playing stream."),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::PlayStart - Sent status messages");
            }

            // Send sample access message

            let sample_access_bytes = rtmp_make_sample_access_message(0, config.chunk_size);

            if let Err(e) = session_write_bytes(write_stream, &sample_access_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send sample access: {}", e));
                }
                return true;
            }

            // Send metadata

            if !metadata.is_empty() {
                let metadata_bytes =
                    rtmp_make_metadata_message(play_stream_id, &metadata, 0, config.chunk_size);

                if let Err(e) = session_write_bytes(write_stream, &metadata_bytes).await {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Send error: Could not send metadata bytes: {}",
                            e
                        ));
                    }
                    return true;
                }

                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("RtmpSessionMessage::PlayStart - Sent metadata message");
                }
            }

            // Send audio codec header

            if audio_codec == 10 || audio_codec == 13 {
                let audio_codec_header = rtmp_make_audio_codec_header_message(
                    play_stream_id,
                    &aac_sequence_header,
                    0,
                    config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &audio_codec_header).await {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Send error: Could not send audio codec header: {}",
                            e
                        ));
                    }
                    return true;
                }

                logger.log_debug("Sent audio codec header");
            }

            // Send video codec header

            if video_codec == 7 || video_codec == 12 {
                let video_codec_header = rtmp_make_video_codec_header_message(
                    play_stream_id,
                    &avc_sequence_header,
                    0,
                    config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &video_codec_header).await {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Send error: Could not send video codec header: {}",
                            e
                        ));
                    }
                    return true;
                }

                logger.log_debug("Sent video codec header");
            }

            // Send GOP cache

            if receive_gop {
                for packet in gop_cache {
                    if packet.header.packet_type == RTMP_TYPE_AUDIO && !receive_audio {
                        continue;
                    }

                    if packet.header.packet_type == RTMP_TYPE_VIDEO && !receive_video {
                        continue;
                    }

                    let packet_bytes =
                        packet.create_chunks_for_stream(play_stream_id, config.chunk_size);

                    if let Err(e) = session_write_bytes(write_stream, &packet_bytes).await {
                        if config.log_requests && logger.config.debug_enabled {
                            logger.log_debug(&format!(
                                "Send error: Could not send GOP cached packet: {}",
                                e
                            ));
                        }
                        return true;
                    }

                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "RtmpSessionMessage::PlayStart - Sent GOP packet: {} bytes",
                            packet.payload.len()
                        ));
                    }
                }
            }

            // Log

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Changed play status: PLAYING");
            }
        }
        RtmpSessionMessage::InvalidKey => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::InvalidKey");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

            if !is_player {
                return true;
            }

            // Set playing status to false
            RtmpSessionStatus::stop_playing(session_status).await;

            // Send status message

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Invalid play stream key provided");
            }

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "error",
                "NetStream.Publish.BadName",
                Some("Invalid stream key provided"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }
        }
        RtmpSessionMessage::PlayMetadata { metadata } => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::PlayMetadata");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

            if !is_player {
                return true;
            }

            if metadata.is_empty() {
                return true;
            }

            // Make metadata message

            let metadata_bytes =
                rtmp_make_metadata_message(play_stream_id, &metadata, 0, config.chunk_size);

            // Send metadata

            if let Err(e) = session_write_bytes(write_stream, &metadata_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not channel metadata: {}", e));
                }
                return true;
            }
        }
        RtmpSessionMessage::PlayPacket { packet } => {
            if config.log_requests && logger.config.trace_enabled {
                logger.log_trace("RtmpSessionMessage::PlayPacket");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

            if !is_player {
                return true;
            }

            let packet_bytes = packet.create_chunks_for_stream(play_stream_id, config.chunk_size);

            if let Err(e) = session_write_bytes(write_stream, &packet_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send packet: {}", e));
                }
                return true;
            }
        }
        RtmpSessionMessage::PlayStop => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::PlayStop");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

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
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_EOF, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send stream status: {}", e));
                }
                return true;
            }

            // Log

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Changed play status: IDLE");
            }
        }
        RtmpSessionMessage::Pause => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::Pause");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_EOF, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send stream status: {}", e));
                }
                return true;
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Pause.Notify",
                Some("Paused live"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            // Log

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Changed play status: PAUSED");
            }
        }
        RtmpSessionMessage::Resume {
            audio_codec,
            aac_sequence_header,
            video_codec,
            avc_sequence_header,
        } => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::Resume");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_BEGIN, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send stream status: {}", e));
                }
                return true;
            }

            // Send audio codec header

            if audio_codec == 10 || audio_codec == 13 {
                let audio_codec_header = rtmp_make_audio_codec_header_message(
                    play_stream_id,
                    &aac_sequence_header,
                    0,
                    config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &audio_codec_header).await {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Send error: Could not send audio codec header: {}",
                            e
                        ));
                    }
                    return true;
                }

                logger.log_debug("Sent audio codec header");
            }

            // Send video codec header

            if video_codec == 7 || video_codec == 12 {
                let video_codec_header = rtmp_make_video_codec_header_message(
                    play_stream_id,
                    &avc_sequence_header,
                    0,
                    config.chunk_size,
                );

                if let Err(e) = session_write_bytes(write_stream, &video_codec_header).await {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Send error: Could not send video codec header: {}",
                            e
                        ));
                    }
                    return true;
                }

                logger.log_debug("Sent video codec header");
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Unpause.Notify",
                Some("Unpaused live"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            // Log

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Changed play status: PLAYING");
            }
        }
        RtmpSessionMessage::ResumeIdle => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::ResumeIdle");
            }

            // Get play status
            let (is_player, play_stream_id) =
                RtmpSessionStatus::get_play_stream_id(session_status).await;

            if !is_player {
                return true;
            }

            // Send stream status

            let stream_status_bytes = rtmp_make_stream_status_message(STREAM_BEGIN, play_stream_id);

            if let Err(e) = session_write_bytes(write_stream, &stream_status_bytes).await {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send stream status: {}", e));
                }
                return true;
            }

            // Send status message

            if let Err(e) = send_status_message(
                write_stream,
                play_stream_id,
                "status",
                "NetStream.Unpause.Notify",
                Some("Unpaused live"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!("Send error: Could not send status message: {}", e));
                }
            }

            // Log

            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Changed play status: IDLE");
            }
        }
        RtmpSessionMessage::Kill => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::Kill");
            }

            RtmpSessionStatus::set_killed(session_status).await;
        }
        RtmpSessionMessage::End => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("RtmpSessionMessage::End");
            }

            return false;
        }
    }

    true
}

/// Creates a task to read and handle session messages
/// session_id - The session ID
/// write_stream - IO stream to write bytes
/// config - Server configuration
/// server_status - Server status
/// session_status - Session status
/// session_msg_receiver - Receiver for the session messages
/// control_key_validator_sender - Sender to communicate with the control server
/// logger - The logger
#[allow(clippy::too_many_arguments)]
pub fn spawn_task_to_read_session_messages<
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    session_id: u64,
    write_stream: Arc<Mutex<TW>>,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    session_status: Arc<Mutex<RtmpSessionStatus>>,
    mut session_msg_receiver: Receiver<RtmpSessionMessage>,
    mut control_key_validator_sender: Option<Sender<ControlKeyValidationRequest>>,
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
                        &write_stream,
                        &config,
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
            &logger,
            session_id,
            &config,
            &server_status,
            &session_status,
            &mut control_key_validator_sender,
        )
        .await;

        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Draining message channel...");
        }

        // Drain channel

        while session_msg_receiver.try_recv().is_ok() {} // Drain the channel to prevent other threads from blocking

        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Completed session messages handling task");
        }
    });
}
