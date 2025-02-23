use crate::rtmp::RtmpPacket;

/// Wrapper for a packet
/// Contains a packet alongside some metadata related to its status
#[derive(Clone)]
pub struct RtmpPacketWrapper {
    /// The packet
    pub packet: RtmpPacket,

    /// Clock value (Used for extended timestamp)
    pub clock: i64,

    /// Current packet size
    pub bytes: usize,

    /// True if the packet was handled
    pub handled: bool,

    // True if used
    pub used: bool,
}

impl RtmpPacketWrapper {
    /// Creates new RtmpPacketWrapper
    pub fn new() -> RtmpPacketWrapper {
        RtmpPacketWrapper{
            packet: RtmpPacket::new_blank(),
            clock: 0,
            bytes: 0,
            handled: false,
            used: false,
        }
    }

    /// Resets the packet wrapper
    pub fn reset(&mut self){
        self.handled = false;
        self.packet.reset_payload();
        self.bytes = 0;
    }

    /// Fully resets the packet wrapper
    pub fn reset_full(&mut self) {
        self.clock = 0;
        self.bytes = 0;
        self.handled = false;
        self.used = false;

        self.packet.reset();
    }
}

