// Logic to handle RTMP sessions

use std::{sync::Arc, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{
    log::Logger,
    rtmp::{generate_s0_s1_s2, RtmpPacket, RTMP_HANDSHAKE_SIZE, RTMP_PING_TIMEOUT, RTMP_VERSION},
    server::RtmpServerContext,
    session::read_rtmp_chunk,
};

use super::{
    session_write_bytes, spawn_task_to_read_session_messages, spawn_task_to_send_pings, RtmpSessionMessage, RtmpSessionReadStatus, SessionContext, SessionReadThreadContext, RTMP_SESSION_MESSAGE_BUFFER_SIZE
};

/// Size if the buffer to store input packets
pub const IN_PACKETS_BUFFER_SIZE: usize = 4;

/// Handles RTMP session
///
/// # Arguments
///
/// * `logger` - The session logger
/// * `server_context` - The server context
/// * `session_context` - The session context
/// * `read_stream` - The stream to read from the client
/// * `write_stream` - The stream to write to the client
pub async fn handle_rtmp_session<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin + 'static,
>(
    logger: Arc<Logger>,
    mut server_context: RtmpServerContext,
    session_context: SessionContext,
    mut read_stream: TR,
    write_stream: Arc<Mutex<TW>>,
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
                if server_context.config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "BAD HANDSHAKE: Could not read initial version byte: {}",
                        e
                    ));
                }
                return;
            }
        },
        Err(_) => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("BAD HANDSHAKE: Could not read initial version byte: Timed out");
            }
            return;
        }
    };

    if version_byte != RTMP_VERSION && server_context.config.log_requests {
        logger.log_error(&format!(
            "BAD HANDSHAKE: Invalid initial version byte. Expected {}, but got {}",
            RTMP_VERSION, version_byte
        ));
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
                if server_context.config.log_requests {
                    logger.log_error(&format!(
                        "BAD HANDSHAKE: Could not read client signature: {}",
                        e
                    ));
                }
                return;
            }
        }
        Err(_) => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("BAD HANDSHAKE: Could not read client signature: Timed out");
            }
            return;
        }
    };

    // Generate and send handshake response to the client

    let handshake_response = match generate_s0_s1_s2(&client_signature, &logger) {
        Ok(r) => r,
        Err(()) => {
            if server_context.config.log_requests {
                logger.log_error("BAD HANDSHAKE: Could not generate handshake response [Note: This is probably a server bug]");
            }
            return;
        }
    };

    if let Err(e) = session_write_bytes(&write_stream, &handshake_response).await {
        if server_context.config.log_requests {
            logger.log_error(&format!(
                "BAD HANDSHAKE: Could not send handshake response: {}",
                e
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
                if server_context.config.log_requests {
                    logger.log_error(&format!(
                        "BAD HANDSHAKE: Could not read client S1 copy: {}",
                        e
                    ));
                }
                return;
            }
        }
        Err(_) => {
            if server_context.config.log_requests && logger.config.debug_enabled {
                logger.log_debug("BAD HANDSHAKE: Could not read client S1 copy: Timed out");
            }
            return;
        }
    };

    if server_context.config.log_requests && logger.config.debug_enabled {
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
        logger.clone(),
        server_context.clone(),
        session_context.clone(),
        write_stream.clone(),
        msg_receiver,
    );

    // Create task to send ping requests

    let (cancel_pings_sender, cancel_pings_receiver) = tokio::sync::mpsc::channel::<()>(1);

    spawn_task_to_send_pings(
        logger.clone(),
        server_context.clone(),
        session_context.clone(),
        write_stream.clone(),
        cancel_pings_receiver,
    );

    // Create array of input packets

    let mut in_packets: [RtmpPacket; IN_PACKETS_BUFFER_SIZE] =
        std::array::from_fn(|_| RtmpPacket::new_blank());

    // Prepare read thread context

    let mut read_thread_context = SessionReadThreadContext{
        id: session_context.id,
        ip: session_context.ip,
        status: session_context.status,
        publish_status: session_context.publish_status,
        session_msg_sender: msg_sender,
        read_status: RtmpSessionReadStatus::new(),
    };

    // Read chunks

    let mut continue_loop = true;

    while continue_loop {
        continue_loop = read_rtmp_chunk(
            &logger,
            &mut server_context,
            &mut read_thread_context,
            &mut read_stream,
            &write_stream,
            &mut in_packets,
        )
        .await;
    }

    // End of loop, make sure all the tasks end

    _ = cancel_pings_sender.send(()).await;
    _ = read_thread_context.session_msg_sender.send(RtmpSessionMessage::End).await;
}
