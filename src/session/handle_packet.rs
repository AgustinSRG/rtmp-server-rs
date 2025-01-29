// Packet handling logic


use std::{cell::RefCell, cmp, rc::Rc, sync::Arc, thread::panicking, time::Duration};

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    log::Logger,
    rtmp::{
        get_rtmp_header_size, RtmpPacket, RTMP_CHUNK_TYPE_0, RTMP_CHUNK_TYPE_1, RTMP_CHUNK_TYPE_2, RTMP_PING_TIMEOUT, RTMP_TYPE_METADATA
    },
    server::{RtmpServerConfiguration, RtmpServerStatus},
};

use super::{RtmpSessionMessage, RtmpSessionPublishStreamStatus, RtmpSessionReadStatus, RtmpSessionStatus};

/// Handles RTMP packet
/// packet - The packet to handle
/// session_id - Session ID
/// read_stream - IO stream to read bytes
/// write_stream - IO stream to write bytes
/// config - RTMP configuration
/// server_status - Server status
/// session_status - Session status
/// publish_status - Status if the stream being published
/// session_msg_sender - Message sender for the session
/// session_msg_receiver - Message receiver for the session
/// read_status - Status for the read task
/// logger - Session logger
/// Return true to continue receiving chunk. Returns false to end the session main loop.
pub async fn handle_rtmp_packet<
    TR: AsyncRead + AsyncReadExt + Send + Sync + Unpin,
    TW: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin,
>(
    packet: &RtmpPacket,
    session_id: u64,
    mut read_stream: &mut TR,
    write_stream: &Mutex<TW>,
    config: &RtmpServerConfiguration,
    server_status: &Mutex<RtmpServerStatus>,
    session_status: &Mutex<RtmpSessionStatus>,
    publish_status: &Arc<Mutex<RtmpSessionPublishStreamStatus>>,
    session_msg_sender: &Sender<RtmpSessionMessage>,
    read_status: &mut RtmpSessionReadStatus,
    logger: &Logger,
) -> bool {
    true
}