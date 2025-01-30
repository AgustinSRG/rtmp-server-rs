// Invoke packet handling logic

use std::sync::Arc;

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use chrono::Utc;
use rustls::quic::PacketKeySet;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{
        get_rtmp_header_size, rtmp_make_chunk_size_set_message, rtmp_make_connect_response,
        rtmp_make_create_stream_response, rtmp_make_peer_bandwidth_set_message,
        rtmp_make_window_ack, RtmpCommand, RtmpPacket, RTMP_CHUNK_SIZE, RTMP_CHUNK_TYPE_0,
        RTMP_CHUNK_TYPE_1, RTMP_CHUNK_TYPE_2, RTMP_MAX_CHUNK_SIZE, RTMP_PEER_BANDWIDTH,
        RTMP_PING_TIMEOUT, RTMP_TYPE_FLEX_MESSAGE, RTMP_TYPE_METADATA, RTMP_TYPE_SET_CHUNK_SIZE,
        RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE, RTMP_WINDOW_ACK,
    },
    server::{RtmpServerConfiguration, RtmpServerStatus},
    utils::validate_id_string,
};

use super::{
    send_status_message, session_write_bytes, RtmpSessionMessage, RtmpSessionPublishStreamStatus,
    RtmpSessionReadStatus, RtmpSessionStatus,
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

/// Handles RTMP command (connect)
/// cmd - The command to handle
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
async fn handle_rtmp_command_connect<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    cmd: &RtmpCommand,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let channel = match cmd.get_argument("cmdObj") {
        Some(cmd_obj) => match cmd_obj.get_object_property("app") {
            Some(app) => {
                let app_str = app.get_string();

                if !validate_id_string(app_str, config.id_max_length) {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!("Command error: Invalid app value: {}", app_str));
                    }

                    return false;
                }

                app_str
            }
            None => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("Command error: app property not provided");
                }

                return false;
            }
        },
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: cmdObj argument not provided");
            }

            return false;
        }
    };

    let object_encoding: Option<u32> = match cmd.get_argument("cmdObj") {
        Some(cmd_obj) => match cmd_obj.get_object_property("objectEncoding") {
            Some(oe) => {
                if !oe.is_undefined() {
                    Some(oe.get_integer() as u32)
                } else {
                    None
                }
            }
            None => None,
        },
        None => None,
    };

    let trans_id = match cmd.get_argument("transId") {
        Some(t) => t.get_integer(),
        None => 0,
    };

    let now = Utc::now().timestamp();

    // Update the session status

    let mut session_status_v = session_status.lock().await;

    if let Some(_) = session_status_v.channel {
        // Already connected. This command is invalid
        drop(session_status_v);
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Connect received, but already connected");
        }

        return false;
    }

    session_status_v.channel = Some(channel.to_string());
    session_status_v.connect_time = now;

    drop(session_status_v);

    // Send window ACK

    let window_ack_bytes = rtmp_make_window_ack(RTMP_WINDOW_ACK);
    if let Err(e) = session_write_bytes(&write_stream, &window_ack_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send window ACK: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Set peer bandwidth

    let peer_bandwidth_bytes = rtmp_make_peer_bandwidth_set_message(RTMP_PEER_BANDWIDTH);
    if let Err(e) = session_write_bytes(&write_stream, &peer_bandwidth_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not set peer bandwidth: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Set chunk size

    let chunk_size_bytes = rtmp_make_chunk_size_set_message(config.chunk_size as u32);
    if let Err(e) = session_write_bytes(&write_stream, &chunk_size_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not set chunk size: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Respond

    let connect_response_bytes =
        rtmp_make_connect_response(trans_id, object_encoding, config.chunk_size);
    if let Err(e) = session_write_bytes(&write_stream, &connect_response_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send connect response: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Done

    true
}

/// Handles RTMP command (createStream)
/// cmd - The command to handle
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
async fn handle_rtmp_command_create_stream<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    cmd: &RtmpCommand,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let trans_id = match cmd.get_argument("transId") {
        Some(t) => t.get_integer(),
        None => 0,
    };

    // Create stream

    let mut session_status_v = session_status.lock().await;
    session_status_v.streams = session_status_v.streams.wrapping_add(1);
    let stream_index = session_status_v.streams as u32;
    drop(session_status_v);

    // Respond

    let response_bytes =
        rtmp_make_create_stream_response(trans_id, stream_index, config.chunk_size);
    if let Err(e) = session_write_bytes(&write_stream, &response_bytes).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send connect response: {}",
                e.to_string()
            ));
        }
        return false;
    }

    // Done

    true
}

/// Handles RTMP command (publish)
/// packet - The packet to handle
/// cmd - The command to handle
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
async fn handle_rtmp_command_publish<TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin>(
    packet: &RtmpPacket,
    cmd: &RtmpCommand,
    session_id: u64,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    logger: &Logger,
) -> bool {
    // Load and validate parameters

    let publish_stream_id = packet.header.stream_id;

    let channel = match RtmpSessionStatus::get_channel(session_status).await {
        Some(c) => c,
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Protocol error: Received publish before connect");
            }

            if let Err(e) = send_status_message(
                &write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadConnection",
                Some("No channel is selected"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Send error: Could not send status message: {}",
                        e.to_string()
                    ));
                }
            }

            return false;
        }
    };

    let key = match cmd.get_argument("streamName") {
        Some(k) => k.get_string(),
        None => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Command error: streamName property not provided");
            }

            if let Err(e) = send_status_message(
                &write_stream,
                publish_stream_id,
                "error",
                "NetStream.Publish.BadName",
                Some("No stream key provided"),
                config.chunk_size,
            )
            .await
            {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Send error: Could not send status message: {}",
                        e.to_string()
                    ));
                }
            }

            return false;
        }
    };

    if !validate_id_string(key, config.id_max_length) {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!("Command error: Invalid streamName value: {}", key));
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Invalid stream key provided"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Ensure the session is not already publishing

    if !RtmpSessionStatus::check_is_publisher(session_status).await {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug("Protocol error: Received publish command, but already publishing");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadConnection",
            Some("Connection already publishing"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Ensure the channel is free to publish

    if !RtmpServerStatus::check_channel_publishing_status(server_status, &channel).await {
        if config.log_requests && logger.config.debug_enabled {
            logger
                .log_debug("Cannot publish: Another session is already publishing on the channel");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Stream already publishing"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Log

    if config.log_requests {
        logger.log_info(&format!("PUBLISH ({}): {}", publish_stream_id, &channel));
    }

    // Check validity of the key (callback or coordinator)

    let stream_id = "";
    logger.log_debug("TODO");

    // Set publisher into the server status

    if !RtmpServerStatus::set_publisher(
        server_status,
        &channel,
        key,
        stream_id,
        session_id,
        publish_status.clone(),
        session_msg_sender.clone(),
    )
    .await
    {
        if config.log_requests && logger.config.debug_enabled {
            logger
                .log_debug("Cannot publish: Another session is already publishing on the channel");
        }

        if let Err(e) = send_status_message(
            &write_stream,
            publish_stream_id,
            "error",
            "NetStream.Publish.BadName",
            Some("Stream already publishing"),
            config.chunk_size,
        )
        .await
        {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug(&format!(
                    "Send error: Could not send status message: {}",
                    e.to_string()
                ));
            }
        }

        return false;
    }

    // Set publishing status to the session status

    RtmpSessionStatus::set_publisher(session_status, true, publish_stream_id).await;

    // Respond with status message

    if let Err(e) = send_status_message(
        &write_stream,
        publish_stream_id,
        "status",
        "NetStream.Publish.Start",
        Some(&format!("/{}/{} is now published.", channel, key)),
        config.chunk_size,
    )
    .await
    {
        if config.log_requests && logger.config.debug_enabled {
            logger.log_debug(&format!(
                "Send error: Could not send status message: {}",
                e.to_string()
            ));
        }
    }

    // Done

    true
}

/// Handles RTMP command (receiveAudio)
/// cmd - The command to handle
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
async fn handle_rtmp_command_receive_audio(
    cmd: &RtmpCommand,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    let receive_audio = match cmd.get_argument("bool") {
        Some(v) => v.get_bool(),
        None => false,
    };

    if config.log_requests && logger.config.debug_enabled {
        logger.log_debug(&format!("Receive audio setting change: {}", receive_audio));
    }

    let mut session_status_v = session_status.lock().await;
    session_status_v.receive_audio = receive_audio;

    let channel_opt = session_status_v.channel.clone();

    drop(session_status_v);

    if let Some(channel) = channel_opt {
        RtmpServerStatus::player_set_receive_audio(
            server_status,
            &channel,
            session_id,
            receive_audio,
        )
        .await;
    }

    true
}

/// Handles RTMP command (receiveVideo)
/// cmd - The command to handle
/// session_id - Session ID
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// logger - Session logger
/// Return true to continue receiving chunks. Returns false to end the session main loop.
async fn handle_rtmp_command_receive_video(
    cmd: &RtmpCommand,
    session_id: u64,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    logger: &Logger,
) -> bool {
    let receive_video = match cmd.get_argument("bool") {
        Some(v) => v.get_bool(),
        None => false,
    };

    if config.log_requests && logger.config.debug_enabled {
        logger.log_debug(&format!("Receive video setting change: {}", receive_video));
    }

    let mut session_status_v = session_status.lock().await;
    session_status_v.receive_video = receive_video;

    let channel_opt = session_status_v.channel.clone();

    drop(session_status_v);

    if let Some(channel) = channel_opt {
        RtmpServerStatus::player_set_receive_video(
            server_status,
            &channel,
            session_id,
            receive_video,
        )
        .await;
    }

    true
}
