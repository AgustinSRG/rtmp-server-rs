// Chunk read logic

use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex}, time::timeout,
};

use crate::{
    log::Logger, rtmp::{get_rtmp_header_size, RTMP_PING_TIMEOUT}, server::{RtmpServerConfiguration, RtmpServerStatus}
};

use super::{RtmpSessionMessage, RtmpSessionReadStatus, RtmpSessionStatus};

/// Reads RTMP chunk and, if ready, handles it
/// session_id - Session ID
/// read_stream - IO stream to read bytes
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// session_msg_sender - Message sender for the session
/// session_msg_receiver - Message receiver for the session
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunk. Returns false to end the session main loop.
pub async fn read_rtmp_chunk<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin,
>(
    session_id: u64,
    mut read_stream: &mut TR,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    read_status: &mut RtmpSessionReadStatus,
    logger: &Logger,
) -> bool {
    let mut bytes_read_count: usize = 0; // Counter for stats

    // Read start byte

    let start_byte = match tokio::time::timeout(Duration::from_secs(RTMP_PING_TIMEOUT), read_stream.read_u8()).await {
        Ok(br) => match br {
            Ok(b) => b,
            Err(e) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Chunk read error. Could not read start byte: {}",
                        e.to_string()
                    ));
                }
                return false;
            }
        },
        Err(_) => {
            if config.log_requests && logger.config.debug_enabled {
                logger.log_debug("Chunk read error. Could not read start byte: Timed out");
            }
            return false;
        }
    };

    bytes_read_count += 1;

    // Read header

    let basic_bytes: usize = if start_byte & 0x3f == 0 {
        2
    } else if start_byte & 0x3f == 1 {
        3
    } else {
       1
    };

    let header_res_bytes_size = get_rtmp_header_size(start_byte >> 6);

    let mut header: Vec<u8> = vec![0; 1 + basic_bytes + header_res_bytes_size];

    header[0] = start_byte;

    for i in 0..basic_bytes {
        let basic_byte = match tokio::time::timeout(Duration::from_secs(RTMP_PING_TIMEOUT), read_stream.read_u8()).await {
            Ok(br) => match br {
                Ok(b) => b,
                Err(e) => {
                    if config.log_requests && logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Chunk read error. Could not read basic byte [{}]: {}",
                            i,
                            e.to_string(),
                        ));
                    }
                    return false;
                }
            },
            Err(_) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug(&format!(
                        "Chunk read error. Could not read basic byte [{}]: Timed out",
                        i
                    ));
                }
                return false;
            }
        };

        header[i + 1] = basic_byte;

        bytes_read_count += 1;
    }

    if header_res_bytes_size > 0 {
        // Read the rest of the header
        match tokio::time::timeout(Duration::from_secs(RTMP_PING_TIMEOUT), read_stream.read_exact(&mut header[1 + basic_bytes..])).await {
            Ok(r) => {
                if let Err(e) = r {
                    if config.log_requests {
                        logger.log_error(&format!(
                            "BAD HANDSHAKE: Could not read client S1 copy: {}",
                            e.to_string()
                        ));
                    }
                    return false;
                }
            },
            Err(_) => {
                if config.log_requests && logger.config.debug_enabled {
                    logger.log_debug("BAD HANDSHAKE: Could not read client S1 copy: Timed out");
                }
                return false;
            }
        };
    }

    // Parse packet metadata




    return true;
}
