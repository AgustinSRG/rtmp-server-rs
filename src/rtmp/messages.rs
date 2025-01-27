// RTMP message generators

use core::time;
use std::collections::HashMap;

use byteorder::{BigEndian, ByteOrder};
use chrono::{DateTime, Utc};

use crate::amf::AMF0Value;

use super::{
    RtmpCommand, RtmpData, RtmpPacket, RTMP_CHANNEL_AUDIO, RTMP_CHANNEL_DATA, RTMP_CHANNEL_INVOKE, RTMP_CHANNEL_PROTOCOL, RTMP_CHANNEL_VIDEO, RTMP_CHUNK_TYPE_0, RTMP_TYPE_AUDIO, RTMP_TYPE_DATA, RTMP_TYPE_EVENT, RTMP_TYPE_INVOKE, RTMP_TYPE_VIDEO
};

/// Makes RTMP ACK message
pub fn rtmp_make_ack(size: u32) -> Vec<u8> {
    let mut b = vec![
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    BigEndian::write_u32(&mut b[12..16], size);

    b
}

/// Makes RTMP window ACK
pub fn rtmp_make_window_ack(size: u32) -> Vec<u8> {
    let mut b = vec![
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    BigEndian::write_u32(&mut b[12..16], size);

    b
}

/// Makes RTMP control message to indicate peer bandwidth
pub fn rtmp_make_peer_bandwidth_set_message(size: u32, t: u8) -> Vec<u8> {
    let mut b = vec![
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];

    BigEndian::write_u32(&mut b[12..16], size);
    b[16] = t;

    b
}

/// Makes RTMP control message to indicate chunk size
pub fn rtmp_make_chunk_size_set_message(size: u32) -> Vec<u8> {
    let mut b = vec![
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    BigEndian::write_u32(&mut b[12..16], size);

    b
}

/// Makes RTMP control message to indicate stream status
/// Use one of these for status: STREAM_BEGIN, STREAM_EOF, STREAM_DRY, STREAM_EMPTY, STREAM_READY
pub fn rtmp_make_stream_status_message(status: u16, stream_id: u32) -> Vec<u8> {
    let mut b = vec![
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00,
    ];

    BigEndian::write_u16(&mut b[12..14], status);
    BigEndian::write_u32(&mut b[14..18], stream_id);

    b
}

/// Makes RTMP ping request message
pub fn rtmp_make_ping_request(connect_time: i64, out_chunk_size: usize) -> Vec<u8> {
    let time: DateTime<Utc> = Utc::now();
    let timestamp = time.timestamp();
    let current_timestamp = timestamp.wrapping_sub(connect_time);

    let mut packet = RtmpPacket::new_blank();

    packet.header.format = RTMP_CHUNK_TYPE_0;
    packet.header.channel_id = RTMP_CHANNEL_PROTOCOL;
    packet.header.packet_type = RTMP_TYPE_EVENT;
    packet.header.timestamp = current_timestamp;

    packet.payload = vec![
        0,
        6,
        ((current_timestamp >> 24) as u8) & 0xff,
        ((current_timestamp >> 16) as u8) & 0xff,
        ((current_timestamp >> 8) as u8) & 0xff,
        (current_timestamp as u8) & 0xff,
    ];

    packet.header.length = packet.payload.len();

    packet.create_chunks(out_chunk_size)
}

/// Makes RTMP invoke command message
pub fn rtmp_make_invoke_message(cmd: &RtmpCommand, stream_id: u32, out_chunk_size: usize) -> Vec<u8> {
    let mut packet = RtmpPacket::new_blank();

    packet.header.format = RTMP_CHUNK_TYPE_0;
    packet.header.channel_id = RTMP_CHANNEL_INVOKE;
    packet.header.packet_type = RTMP_TYPE_INVOKE;
    packet.header.stream_id = stream_id;
    packet.payload = cmd.encode();
    packet.header.length = packet.payload.len();

    packet.create_chunks(out_chunk_size)
}

/// Makes RTMP data message
pub fn rtmp_make_data_message(data: &RtmpData, stream_id: u32, out_chunk_size: usize) -> Vec<u8> {
    let mut packet = RtmpPacket::new_blank();

    packet.header.format = RTMP_CHUNK_TYPE_0;
    packet.header.channel_id = RTMP_CHANNEL_DATA;
    packet.header.packet_type = RTMP_TYPE_DATA;
    packet.header.stream_id = stream_id;
    packet.payload = data.encode();
    packet.header.length = packet.payload.len();

    packet.create_chunks(out_chunk_size)
}

/// Makes RTMP status message
pub fn rtmp_make_status_message(
    stream_id: u32,
    level: String,
    code: String,
    description: Option<String>,
    out_chunk_size: usize,
) -> Vec<u8> {
    let mut cmd = RtmpCommand::new("onStatus".to_string());

    cmd.set_argument("transId".to_string(), AMF0Value::Number { value: 0.0 });
    cmd.set_argument("cmdObj".to_string(), AMF0Value::Null);

    let mut info: HashMap<String, AMF0Value> = HashMap::new();

    info.insert("level".to_string(), AMF0Value::String { value: level });
    info.insert("code".to_string(), AMF0Value::String { value: code });

    if let Some(d) = description {
        info.insert("description".to_string(), AMF0Value::String { value: d });
    }

    cmd.set_argument("info".to_string(), AMF0Value::Object { properties: info });

    rtmp_make_invoke_message(&cmd, stream_id, out_chunk_size)
}

/// Makes RTMP sample access message
pub fn rtmp_make_sample_access_message(stream_id: u32, out_chunk_size: usize) -> Vec<u8> {
    let mut data = RtmpData::new("|RtmpSampleAccess".to_string());

    data.set_argument("bool1".to_string(), AMF0Value::Bool { value: false });
    data.set_argument("bool2".to_string(), AMF0Value::Bool { value: false });

    rtmp_make_data_message(&data, stream_id, out_chunk_size)
}

/// Makes message to respond to a connect message
pub fn rtmp_make_connect_response(
    trans_id: i64,
    object_encoding: Option<u32>,
    out_chunk_size: usize,
) -> Vec<u8> {
    let mut cmd = RtmpCommand::new("_result".to_string());

    cmd.set_argument(
        "transId".to_string(),
        AMF0Value::Number { value: trans_id as f64 },
    );

    let mut cmd_obj: HashMap<String, AMF0Value> = HashMap::new();

    cmd_obj.insert("fmsVer".to_string(), AMF0Value::String { value: "FMS/3,0,1,123".to_string() });
    cmd_obj.insert("capabilities".to_string(), AMF0Value::Number { value: 31.0 });

    cmd.set_argument(
        "cmdObj".to_string(),
        AMF0Value::Object {
            properties: cmd_obj,
        },
    );

    let mut info: HashMap<String, AMF0Value> = HashMap::new();

    info.insert(
        "level".to_string(),
        AMF0Value::String {
            value: "status".to_string(),
        },
    );
    info.insert(
        "code".to_string(),
        AMF0Value::String {
            value: "NetConnection.Connect.Success".to_string(),
        },
    );
    info.insert(
        "description".to_string(),
        AMF0Value::String {
            value: "Connection succeeded.".to_string(),
        },
    );

    match object_encoding {
        Some(oe) => {
            info.insert(
                "objectEncoding".to_string(),
                AMF0Value::Number { value: oe as f64 },
            );
        }
        None => {
            info.insert("objectEncoding".to_string(), AMF0Value::Undefined);
        }
    }

    cmd.set_argument("info".to_string(), AMF0Value::Object { properties: info });

    rtmp_make_invoke_message(&cmd, 0, out_chunk_size)
}


/// Makes message to respond to a connect message
pub fn rtmp_make_create_stream_response(
    trans_id: i64,
    stream_index: u32,
    out_chunk_size: usize,
) -> Vec<u8> {
    let mut cmd = RtmpCommand::new("_result".to_string());

    cmd.set_argument(
        "transId".to_string(),
        AMF0Value::Number { value: trans_id as f64 },
    );

    cmd.set_argument(
        "cmdObj".to_string(),
        AMF0Value::Null,
    );

    cmd.set_argument("info".to_string(), AMF0Value::Number { value: stream_index as f64 });

    rtmp_make_invoke_message(&cmd, 0, out_chunk_size)
}

/// Creates metadata message (used to send stream metadata to clients)
pub fn rtmp_make_metadata_message(play_stream_id: u32, metadata: &[u8], timestamp: i64, out_chunk_size: usize) -> Vec<u8> {
    let mut packet = RtmpPacket::new_blank();

    packet.header.format = RTMP_CHUNK_TYPE_0;
    packet.header.channel_id = RTMP_CHANNEL_DATA;
    packet.header.packet_type = RTMP_TYPE_DATA;
    packet.header.stream_id = play_stream_id;

    let mut payload: Vec<u8> = Vec::with_capacity(metadata.len());
    payload[..].copy_from_slice(metadata);
    packet.payload = payload;

    packet.header.length = packet.payload.len();

    packet.header.timestamp = timestamp;

    packet.create_chunks(out_chunk_size)
}

/// Creates RTMP audio codec header message
pub fn rtmp_make_audio_codec_header_message(play_stream_id: u32, aac_sequence_header: &[u8], timestamp: i64, out_chunk_size: usize) -> Vec<u8> {
    let mut packet = RtmpPacket::new_blank();

    packet.header.format = RTMP_CHUNK_TYPE_0;
    packet.header.channel_id = RTMP_CHANNEL_AUDIO;
    packet.header.packet_type = RTMP_TYPE_AUDIO;
    packet.header.stream_id = play_stream_id;

    let mut payload: Vec<u8> = Vec::with_capacity(aac_sequence_header.len());
    payload[..].copy_from_slice(aac_sequence_header);
    packet.payload = payload;

    packet.header.length = packet.payload.len();

    packet.header.timestamp = timestamp;

    packet.create_chunks(out_chunk_size)
}

/// Creates RTMP video codec header message
pub fn rtmp_make_video_codec_header_message(play_stream_id: u32, avc_sequence_header: &[u8], timestamp: i64, out_chunk_size: usize) -> Vec<u8> {
    let mut packet = RtmpPacket::new_blank();

    packet.header.format = RTMP_CHUNK_TYPE_0;
    packet.header.channel_id = RTMP_CHANNEL_VIDEO;
    packet.header.packet_type = RTMP_TYPE_VIDEO;
    packet.header.stream_id = play_stream_id;

    let mut payload: Vec<u8> = Vec::with_capacity(avc_sequence_header.len());
    payload[..].copy_from_slice(avc_sequence_header);
    packet.payload = payload;

    packet.header.length = packet.payload.len();

    packet.header.timestamp = timestamp;

    packet.create_chunks(out_chunk_size)
}

/// Build RTMP metadata to be stored in order to send to players
pub fn rtmp_build_metadata(data: &RtmpData) -> Vec<u8> {
    let mut res = RtmpData::new("onMetaData".to_string());

    let arg_res = data.get_argument("dataObj");

    match arg_res {
        Some(arg) => {
            res.set_argument("dataObj".to_string(), arg.clone());
        }
        None => {
            res.set_argument("dataObj".to_string(), AMF0Value::Null);
        },
    }

    res.encode()
}
