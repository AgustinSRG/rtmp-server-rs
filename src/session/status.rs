// RTMP session status model

use std::{collections::HashMap, net::IpAddr};

use crate::rtmp::RtmpPacket;

/// RTMP session status
pub struct RtmpSessionStatus {
    /// Session ID
    pub id: u64,

    /// Client IP address
    pub ip: IpAddr,
}

impl RtmpSessionStatus {
    pub fn new(id: u64, ip: IpAddr) -> RtmpSessionStatus {
        RtmpSessionStatus { id, ip }
    }
}

/// Status to maintain only for the read task
pub struct RtmpSessionReadStatus {
    /// Packets being build during read
    pub in_packets: HashMap<u32, RtmpPacket>,
}

impl RtmpSessionReadStatus {
    /// Creates RtmpSessionReadStatus
    pub fn new() -> RtmpSessionReadStatus {
        RtmpSessionReadStatus {
            in_packets: HashMap::new(),
        }
    }
}
