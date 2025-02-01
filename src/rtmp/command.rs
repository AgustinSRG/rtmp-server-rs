// RTMP command

use std::{collections::HashMap, sync::LazyLock};

use crate::amf::{AMF0Value, AMFDecodingCursor};

/// RTMP command
pub struct RtmpCommand {
    /// Command
    pub cmd: String,

    /// Arguments
    pub arguments: HashMap<String, AMF0Value>,
}

static RTMP_COMMAND_CODES: LazyLock<HashMap<String, Vec<String>>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert(
        "_result".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "info".to_string(),
        ],
    );

    m.insert(
        "_error".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "info".to_string(),
            "streamId".to_string(),
        ],
    );

    m.insert(
        "onStatus".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "info".to_string(),
        ],
    );

    m.insert(
        "releaseStream".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamName".to_string(),
        ],
    );

    m.insert(
        "getStreamLength".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamId".to_string(),
        ],
    );

    m.insert(
        "getMovLen".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamId".to_string(),
        ],
    );

    m.insert(
        "FCPublish".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamName".to_string(),
        ],
    );

    m.insert(
        "FCUnpublish".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamName".to_string(),
        ],
    );

    m.insert(
        "FCSubscribe".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamName".to_string(),
        ],
    );

    m.insert(
        "onFCPublish".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "info".to_string(),
        ],
    );

    m.insert(
        "connect".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "args".to_string(),
        ],
    );

    m.insert(
        "call".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "args".to_string(),
        ],
    );

    m.insert(
        "createStream".to_string(),
        vec!["transId".to_string(), "cmdObj".to_string()],
    );

    m.insert(
        "close".to_string(),
        vec!["transId".to_string(), "cmdObj".to_string()],
    );

    m.insert(
        "play".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamName".to_string(),
            "start".to_string(),
            "duration".to_string(),
            "reset".to_string(),
        ],
    );

    m.insert(
        "play2".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "params".to_string(),
        ],
    );

    m.insert(
        "deleteStream".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamId".to_string(),
        ],
    );

    m.insert(
        "closeStream".to_string(),
        vec!["transId".to_string(), "cmdObj".to_string()],
    );

    m.insert(
        "receiveAudio".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "bool".to_string(),
        ],
    );

    m.insert(
        "receiveVideo".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "bool".to_string(),
        ],
    );

    m.insert(
        "publish".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "streamName".to_string(),
            "type".to_string(),
        ],
    );

    m.insert(
        "seek".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "ms".to_string(),
        ],
    );

    m.insert(
        "pause".to_string(),
        vec![
            "transId".to_string(),
            "cmdObj".to_string(),
            "pause".to_string(),
            "ms".to_string(),
        ],
    );

    m
});

impl RtmpCommand {
    /// Creates RtmpCommand
    pub fn new(cmd: String) -> RtmpCommand {
        RtmpCommand{
            cmd,
            arguments: HashMap::new(),
        }
    }

    /// Sets argument
    pub fn set_argument(&mut self, arg_name: String, value: AMF0Value) {
        self.arguments.insert(arg_name, value);
    }

    /// Gets argument
    pub fn get_argument(&self, arg_name: &str) -> Option<&AMF0Value> {
        self.arguments.get(arg_name)
    }

    /// Gets string representation of the command for debug logging
    pub fn to_debug_string(&self) -> String {
        let mut s = format!("{} {}\n", self.cmd, "{");

        for (arg_name, arg_val) in &self.arguments {
            s.push_str(&format!("    '{}' = {}\n", arg_name, arg_val.to_debug_string("    ")));
        }

        s.push('}');

        s
    }

    /// Encodes command
    pub fn encode(&self) -> Vec<u8> {
        let x = AMF0Value::String{
            value: self.cmd.clone(),
        };

        let mut buf = x.encode();

        let arg_list_res = RTMP_COMMAND_CODES.get(&self.cmd);

        if let Some(arg_list) = arg_list_res {
            for arg_name in arg_list {
                let val_res = self.arguments.get(arg_name);

                match val_res {
                    Some(val) => {
                        buf.extend(val.encode());
                    },
                    None => {
                        buf.extend(AMF0Value::Undefined.encode());
                    },
                }
            }
        }

        buf
    }

    /// Decodes command from bytes
    pub fn decode(data: &[u8]) -> Result<RtmpCommand, ()> {
        let mut cursor = AMFDecodingCursor::new(data);

        let cmd_amf = AMF0Value::read(&mut cursor, data)?;
        let cmd = cmd_amf.get_string();

        let mut c = RtmpCommand::new(cmd.to_string());

        let arg_list_res = RTMP_COMMAND_CODES.get(cmd);

        if let Some(arg_list) = arg_list_res {
            let mut i: usize =  0;

            while i < arg_list.len() && !cursor.ended() {
                let val = AMF0Value::read(&mut cursor, data)?;

                c.set_argument(arg_list[i].clone(), val);

                i += 1;
            }
        }


        Ok(c)
    }
}
