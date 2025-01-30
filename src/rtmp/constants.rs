// RTMP constants

/// RTMP version
pub const RTMP_VERSION: u8 = 3;

/// Handshake size
pub const RTMP_HANDSHAKE_SIZE: usize = 1536;

// Message formats
pub const MESSAGE_FORMAT_0: u32 = 0;
pub const MESSAGE_FORMAT_1: u32 = 1;
pub const MESSAGE_FORMAT_2: u32 = 2;

/// Signature size
pub const RTMP_SIG_SIZE: usize = 1536;

/// SHA 256 size
pub const SHA256DL: usize = 32;
pub const SHA256K: usize = 32;

/// Random CRUD data used for handshake
pub const RANDOM_CRUD: &[u8] = &[
    0xf0, 0xee, 0xc2, 0x4a, 0x80, 0x68, 0xbe, 0xe8,
	0x2e, 0x00, 0xd0, 0xd1, 0x02, 0x9e, 0x7e, 0x57,
	0x6e, 0xec, 0x5d, 0x2d, 0x29, 0x80, 0x6f, 0xab,
	0x93, 0xb8, 0xe6, 0x36, 0xcf, 0xeb, 0x31, 0xae,
];

/// Flash media player name
pub const GENUINE_FMS: &str = "Genuine Adobe Flash Media Server 001";

/// Flash player name
pub const GENUINE_FP: &str = "Genuine Adobe Flash Player 001";

// Chunk types
pub const RTMP_CHUNK_TYPE_0: u32 = 0; // 11-bytes: timestamp(3) + length(3) + stream type(1) + stream id(4)
pub const RTMP_CHUNK_TYPE_1: u32 = 1; // 7-bytes: delta(3) + length(3) + stream type(1)
pub const RTMP_CHUNK_TYPE_2: u32 = 2; // 3-bytes: delta(3)
pub const RTMP_CHUNK_TYPE_3: u32 = 3; // 0-byte

// RTMP channel types
pub const RTMP_CHANNEL_PROTOCOL: u32 = 2;
pub const RTMP_CHANNEL_INVOKE: u32 = 3;
pub const RTMP_CHANNEL_AUDIO: u32 = 4;
pub const RTMP_CHANNEL_VIDEO: u32 = 5;
pub const RTMP_CHANNEL_DATA: u32 = 6;

/// Gets RTMP header size from the first byte
pub fn get_rtmp_header_size(header_byte: u8) -> usize {
    match header_byte {
        0 => 11,
        1 => 7,
        2 => 3,
        _ => 0,
    }
}

// Packet types

/* Protocol Control Messages */
pub const RTMP_TYPE_SET_CHUNK_SIZE: u32 = 1;
pub const RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE: u32 = 5; // server bandwidth

/* User Control Messages Event (4) */
pub const RTMP_TYPE_EVENT: u32 = 4;

pub const RTMP_TYPE_AUDIO: u32 = 8;
pub const RTMP_TYPE_VIDEO: u32 = 9;

/* Data Message */
pub const RTMP_TYPE_FLEX_STREAM: u32 = 15; // AMF3
pub const RTMP_TYPE_DATA: u32 = 18; // AMF0

/* Command Message */
pub const RTMP_TYPE_FLEX_MESSAGE: u32 = 17; // AMF3
pub const RTMP_TYPE_INVOKE: u32 = 20; // AMF0

/* Aggregate Message */
pub const RTMP_TYPE_METADATA: u32 = 22;

// Stream statuses

pub const STREAM_BEGIN: u16 = 0x00;
pub const STREAM_EOF: u16 = 0x01;

/// Default chunk size
pub const RTMP_CHUNK_SIZE: usize = 128;

/// Max chunk size
pub const RTMP_MAX_CHUNK_SIZE: usize = 65536;

/// Ping time (seconds)
pub const RTMP_PING_TIME: u64 = 30;

/// Ping timeout (seconds)
pub const RTMP_PING_TIMEOUT: u64 = 60;

/// Window ACK
pub const RTMP_WINDOW_ACK: u32 = 5000000;

/// Peer bandwidth
pub const RTMP_PEER_BANDWIDTH: u32 = 5000000;

/// Base size of a RTMP packet
pub const RTMP_PACKET_BASE_SIZE: usize = 65;
