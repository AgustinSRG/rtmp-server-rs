// Logic to handle RTMP sessions

use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    control::ControlKeyValidationRequest, log::Logger, rtmp::{generate_s0_s1_s2, RtmpPacket, RTMP_HANDSHAKE_SIZE, RTMP_PING_TIMEOUT, RTMP_VERSION}, server::{RtmpServerConfiguration, RtmpServerStatus}, session::read_rtmp_chunk
};

use super::{
    session_write_bytes, spawn_task_to_read_session_messages, spawn_task_to_send_pings,
    RtmpSessionMessage, RtmpSessionPublishStreamStatus, RtmpSessionReadStatus, RtmpSessionStatus,
    RTMP_SESSION_MESSAGE_BUFFER_SIZE,
};

/// Handles RTMP session
/// session_id - Session ID
/// ip - Client IP address
/// connection - IO stream to read and write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// control_key_validator_sender - Sender for key validation against the control server
/// logger - Session logger
pub async fn handle_rtmp_session<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin +'static ,
>(
    session_id: u64,
    ip: IpAddr,
    mut read_stream: TR,
    write_stream: Arc<Mutex<TW>>,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    session_status: Arc<Mutex<RtmpSessionStatus>>,
    publish_status: Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    mut control_key_validator_sender: Option<Sender<ControlKeyValidationRequest>>,
    logger: Arc<Logger>,
) {
    ////////////////////
    //    Handshake   //
    ////////////////////

    // Start by reading initial byte (protocol version)

    let version_byte = match tokio::time::timeout(
        Duration::from_secs(RTMP_PING_TIMEOUT),
        read_stream.read_u8(),
    )
    .await
    {
        Ok(br) => match br {
            Ok(b) => b,
            Err(e) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "BAD HANDSHAKE: Could not read initial version byte: {}",
                        e.to_string()
                    ));
                }
                return;
            }
        },
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("BAD HANDSHAKE: Could not read initial version byte: Timed out");
            }
            return;
        }
    };

    if version_byte != RTMP_VERSION {
        if config.log_requests {
            logger.log_error(&format!(
                "BAD HANDSHAKE: Invalid initial version byte. Expected {}, but got {}",
                RTMP_VERSION, version_byte
            ));
        }
    }

    // Now, read client signature bytes

    let mut client_signature: Vec<u8> = vec![0; RTMP_HANDSHAKE_SIZE];

    match tokio::time::timeout(
        Duration::from_secs(RTMP_PING_TIMEOUT),
        read_stream.read_exact(&mut client_signature),
    )
    .await
    {
        Ok(r) => {
            if let Err(e) = r {
                if config.log_requests {
                    logger.log_error(&format!(
                        "BAD HANDSHAKE: Could not read client signature: {}",
                        e.to_string()
                    ));
                }
                return;
            }
        }
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("BAD HANDSHAKE: Could not read client signature: Timed out");
            }
            return;
        }
    };

    // Generate and send handshake response to the client

    let handshake_response = match generate_s0_s1_s2(&client_signature, &logger) {
        Ok(r) => r,
        Err(()) => {
            if config.log_requests {
                logger.log_error("BAD HANDSHAKE: Could not generate handshake response [Note: This is probably a server bug]");
            }
            return;
        }
    };

    if let Err(e) = session_write_bytes(&write_stream, &handshake_response).await {
        if config.log_requests {
            logger.log_error(&format!(
                "BAD HANDSHAKE: Could not send handshake response: {}",
                e.to_string()
            ));
        }
        return;
    }

    // Now, the client should send a copy of S1 back, read it, and ignore it

    match tokio::time::timeout(
        Duration::from_secs(RTMP_PING_TIMEOUT),
        read_stream.read_exact(&mut client_signature),
    )
    .await
    {
        Ok(r) => {
            if let Err(e) = r {
                if config.log_requests {
                    logger.log_error(&format!(
                        "BAD HANDSHAKE: Could not read client S1 copy: {}",
                        e.to_string()
                    ));
                }
                return;
            }
        }
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("BAD HANDSHAKE: Could not read client S1 copy: Timed out");
            }
            return;
        }
    };

    if config.log_requests && logger.config.debug_enabled {
        logger.log_debug("Handshake successful. Entering main loop...");
    }

    ////////////////////
    //    Main loop   //
    ////////////////////

    // Create channel for session messages

    let (msg_sender, msg_receiver) =
        tokio::sync::mpsc::channel::<RtmpSessionMessage>(RTMP_SESSION_MESSAGE_BUFFER_SIZE);

    // Create a task to read messages

    spawn_task_to_read_session_messages(
        session_id,
        write_stream.clone(),
        config.clone(),
        server_status.clone(),
        session_status.clone(),
        msg_receiver,
        control_key_validator_sender.clone(),
        logger.clone(),
    );

    // Create task to send ping requests

    let (cancel_pings_sender, cancel_pings_receiver) = tokio::sync::mpsc::channel::<()>(1);

    spawn_task_to_send_pings(
        write_stream.clone(),
        config.clone(),
        session_status.clone(),
        cancel_pings_receiver,
        logger.clone(),
    );

    // Create data too keep between chunk reads

    let mut read_status = RtmpSessionReadStatus::new(ip);
    let mut in_packets: HashMap<u32, RtmpPacket> = HashMap::new();

    // Read chunks

    let mut continue_loop = true;

    while continue_loop {
        continue_loop = read_rtmp_chunk(
            session_id,
            &mut read_stream,
            &write_stream,
            &config,
            &server_status,
            &session_status,
            &publish_status,
            &msg_sender,
            &mut read_status,
            &mut in_packets,
            &mut control_key_validator_sender,
            &logger,
        )
        .await;
    }

    // End of loop, make sure all the tasks end

    _ = cancel_pings_sender.send(()).await;
    _ = msg_sender.send(RtmpSessionMessage::End).await;
}
