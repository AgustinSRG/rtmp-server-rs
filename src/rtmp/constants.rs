// RTMP constants

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
pub fn get_rtmp_header_size(header_byte: u8) -> u32 {
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
pub const RTMP_TYPE_ABORT: u32 = 2;
pub const RTMP_TYPE_ACKNOWLEDGEMENT: u32 = 3; // bytes read report
pub const RTMP_TYPE_WINDOW_ACKNOWLEDGEMENT_SIZE: u32 = 5; // server bandwidth
pub const RTMP_TYPE_SET_PEER_BANDWIDTH: u32 = 6; // client bandwidth

/* User Control Messages Event (4) */
pub const RTMP_TYPE_EVENT: u32 = 4;

pub const RTMP_TYPE_AUDIO: u32 = 8;
pub const RTMP_TYPE_VIDEO: u32 = 9;

/* Data Message */
pub const RTMP_TYPE_FLEX_STREAM: u32 = 15; // AMF3
pub const RTMP_TYPE_DATA: u32 = 18; // AMF0

/* Shared Object Message */
pub const RTMP_TYPE_FLEX_OBJECT: u32 = 16; // AMF3
pub const RTMP_TYPE_SHARED_OBJECT: u32 = 19; // AMF0

/* Command Message */
pub const RTMP_TYPE_FLEX_MESSAGE: u32 = 17; // AMF3
pub const RTMP_TYPE_INVOKE: u32 = 20; // AMF0

/* Aggregate Message */
pub const RTMP_TYPE_METADATA: u32 = 22;

// Stream statuses

pub const STREAM_BEGIN: u16 = 0x00;
pub const STREAM_EOF: u16 = 0x01;
pub const STREAM_DRY: u16 = 0x02;
pub const STREAM_EMPTY: u16 = 0x1f;
pub const STREAM_READY: u16 = 0x20;
