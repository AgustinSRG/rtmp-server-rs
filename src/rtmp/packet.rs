// RTMP packet model

use byteorder::{BigEndian, ByteOrder, LittleEndian};

use super::{
    RTMP_CHUNK_TYPE_0, RTMP_CHUNK_TYPE_1, RTMP_CHUNK_TYPE_2, RTMP_CHUNK_TYPE_3,
    RTMP_PACKET_BASE_SIZE,
};

/// Header of an RTMP packet
#[derive(Clone)]
pub struct RtmpPacketHeader {
    /// Timestamp
    pub timestamp: i64,

    /// Packet format
    pub format: u32,

    /// Channel ID
    pub channel_id: u32,

    /// Packet type
    pub packet_type: u32,

    /// Stream ID
    pub stream_id: u32,

    // Payload length
    pub length: usize,
}

impl RtmpPacketHeader {
    /// Resets the header
    pub fn reset(&mut self) {
        *self = RtmpPacketHeader {
            timestamp: 0,
            format: 0,
            channel_id: 0,
            packet_type: 0,
            stream_id: 0,
            length: 0,
        };
    }
}

/// RTMP packet
#[derive(Clone)]
pub struct RtmpPacket {
    /// Packet header
    pub header: RtmpPacketHeader,

    /// Clock value (Used for extended timestamp)
    pub clock: i64,

    /// Current packet size
    pub bytes: usize,

    /// True if the packet was handled
    pub handled: bool,

    // True if used
    pub used: bool,

    /// Packet payload
    pub payload: Vec<u8>,
}

impl RtmpPacket {
    /// Creates new blank RTMP packet
    pub fn new_blank() -> RtmpPacket {
        RtmpPacket {
            header: RtmpPacketHeader {
                timestamp: 0,
                format: 0,
                channel_id: 0,
                packet_type: 0,
                stream_id: 0,
                length: 0,
            },
            clock: 0,
            bytes: 0,
            handled: false,
            used: false,
            payload: Vec::new(),
        }
    }

    /// Resets the payload and sets handled to false
    pub fn reset(&mut self) {
        self.handled = false;
        self.payload.truncate(0);
        self.bytes = 0;
    }

    /// Fully resets the packet
    pub fn reset_full(&mut self) {
        self.header.reset();
        self.clock = 0;
        self.bytes = 0;
        self.handled = false;
        self.used = false;
        self.payload = Vec::new();
    }

    /// Gets packet total size
    pub fn size(&self) -> usize {
        self.payload.len().wrapping_add(RTMP_PACKET_BASE_SIZE)
    }

    /// Serializes a basic header for a RTMP packet
    /// fmt - Packet format
    /// cid - Packet channel ID
    /// Returns the serialized bytes
    pub fn serialize_basic_header(format: u32, channel_id: u32) -> Vec<u8> {
        if channel_id >= 64 + 255 {
            vec![
                ((format << 6) as u8) | 1,
                ((channel_id - 64) as u8),
                (((channel_id - 64) >> 8) as u8),
            ]
        } else if channel_id >= 64 {
            vec![(format << 6) as u8, ((channel_id - 64) as u8)]
        } else {
            vec![((format << 6) as u8) | (channel_id as u8)]
        }
    }

    /// Serializes the header of a RTMP packet
    /// Returns the serialized bytes
    pub fn serialize_chunk_message_header(&self, stream_id: u32) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();

        if self.header.format <= RTMP_CHUNK_TYPE_2 {
            let mut b: Vec<u8> = vec![0; 4];

            if self.header.timestamp >= 0xffffff {
                BigEndian::write_u32(&mut b, 0xffffff);
            } else {
                BigEndian::write_u32(&mut b, self.header.timestamp as u32);
            }

            out.extend(&b[1..]);
        }

        if self.header.format <= RTMP_CHUNK_TYPE_1 {
            let mut b: Vec<u8> = vec![0; 4];

            BigEndian::write_u32(&mut b, self.header.length as u32);

            out.extend(&b[1..]);
            out.push(self.header.packet_type as u8);
        }

        if self.header.format == RTMP_CHUNK_TYPE_0 {
            let mut b: Vec<u8> = vec![0; 4];

            LittleEndian::write_u32(&mut b, stream_id);

            out.extend(b);
        }

        out
    }

    /// Creates the chunks for an RTMP packet
    /// out_chunk_size - Size of the output chunks
    pub fn create_chunks(&self, out_chunk_size: usize) -> Vec<u8> {
        self.create_chunks_for_stream(self.header.stream_id, out_chunk_size)
    }

    /// Creates the chunks for an RTMP packet
    /// stream_id - Stream ID
    /// out_chunk_size - Size of the output chunks
    pub fn create_chunks_for_stream(&self, stream_id: u32, out_chunk_size: usize) -> Vec<u8> {
        let chunk_basic_header =
            Self::serialize_basic_header(self.header.format, self.header.channel_id);

        let chunk_basic_header_3 =
            Self::serialize_basic_header(RTMP_CHUNK_TYPE_3, self.header.channel_id);

        let chunk_message_header = self.serialize_chunk_message_header(stream_id);

        let use_extended_timestamp = self.header.timestamp >= 0xffffff;

        let mut header_size = chunk_basic_header.len() + chunk_message_header.len();
        let mut payload_size = self.header.length;

        if payload_size > self.payload.len() {
            payload_size = self.payload.len();
        }

        let mut chunks_offset: usize = 0;
        let mut payload_offset: usize = 0;

        if use_extended_timestamp {
            header_size += 4;
        }

        let mut n = header_size + payload_size + (payload_size / out_chunk_size);

        if use_extended_timestamp {
            n += (payload_size / out_chunk_size) * 4
        }

        if payload_size > 0 && payload_size % out_chunk_size == 0 {
            n -= 1;

            if use_extended_timestamp {
                n -= 4;
            }
        }

        let mut chunks: Vec<u8> = vec![0; n];

        chunks[chunks_offset..chunks_offset + chunk_basic_header.len()]
            .copy_from_slice(&chunk_basic_header);

        chunks_offset += chunk_basic_header.len();

        chunks[chunks_offset..chunks_offset + chunk_message_header.len()]
            .copy_from_slice(&chunk_message_header);

        chunks_offset += chunk_message_header.len();

        if use_extended_timestamp {
            BigEndian::write_u32(
                &mut chunks[chunks_offset..chunks_offset + 4],
                self.header.timestamp as u32,
            );
            chunks_offset += 4;
        }

        while payload_size > 0 {
            if payload_size > out_chunk_size {
                let sub_payload = &self.payload[payload_offset..payload_offset + out_chunk_size];

                chunks[chunks_offset..chunks_offset + sub_payload.len()]
                    .copy_from_slice(sub_payload);

                payload_size -= out_chunk_size;
                chunks_offset += out_chunk_size;
                payload_offset += out_chunk_size;

                chunks[chunks_offset..chunks_offset + chunk_basic_header_3.len()]
                    .copy_from_slice(&chunk_basic_header_3);

                chunks_offset += chunk_basic_header_3.len();

                if use_extended_timestamp {
                    BigEndian::write_u32(
                        &mut chunks[chunks_offset..chunks_offset + 4],
                        self.header.timestamp as u32,
                    );
                    chunks_offset += 4;
                }
            } else {
                let sub_payload = &self.payload[payload_offset..payload_offset + payload_size];

                chunks[chunks_offset..chunks_offset + sub_payload.len()]
                    .copy_from_slice(sub_payload);

                payload_size = 0;
                chunks_offset += payload_size;
                payload_offset += payload_size;
            }
        }

        chunks
    }
}
