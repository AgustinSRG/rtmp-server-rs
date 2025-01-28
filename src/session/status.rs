// RTMP session status model

use std::net::IpAddr;



/// RTMP session status
pub struct RtmpSessionStatus {
    /// Session ID
    pub id: u64,

    /// Client IP address
    pub ip:  IpAddr,
}


impl RtmpSessionStatus {
    pub fn new(id: u64, ip: IpAddr) -> RtmpSessionStatus {
        RtmpSessionStatus{
            id,
            ip
        }
    }
}
