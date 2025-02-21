// RTMP data

use std::{collections::HashMap, sync::LazyLock};

use crate::amf::{AMF0Value, AMFDecodingCursor};

/// RTMP data
pub struct RtmpData {
    /// Data tag
    pub tag: String,

    /// Arguments
    pub arguments: HashMap<String, AMF0Value>,
}

static RTMP_DATA_CODES: LazyLock<HashMap<String, Vec<String>>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert(
        "@setDataFrame".to_string(),
        vec!["method".to_string(), "dataObj".to_string()],
    );

    m.insert("onFI".to_string(), vec!["info".to_string()]);

    m.insert("onMetaData".to_string(), vec!["dataObj".to_string()]);

    m.insert(
        "|RtmpSampleAccess".to_string(),
        vec!["bool1".to_string(), "bool2".to_string()],
    );

    m
});

impl RtmpData {
    /// Creates RtmpData
    pub fn new(tag: String) -> RtmpData {
        RtmpData {
            tag,
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
        let mut s = format!("{} {}\n", self.tag, "{");

        for (arg_name, arg_val) in &self.arguments {
            s.push_str(&format!(
                "    '{}' = {}\n",
                arg_name,
                arg_val.to_debug_string("    ")
            ));
        }

        s.push('}');

        s
    }

    /// Encodes data
    pub fn encode(&self) -> Vec<u8> {
        let x = AMF0Value::String {
            value: self.tag.clone(),
        };

        let mut buf = x.encode();

        let arg_list_res = RTMP_DATA_CODES.get(&self.tag);

        if let Some(arg_list) = arg_list_res {
            for arg_name in arg_list {
                let val_res = self.arguments.get(arg_name);

                if let Some(val) = val_res {
                    buf.extend(val.encode());
                }
            }
        }

        buf
    }

    /// Decodes data from bytes
    pub fn decode(data: &[u8]) -> Result<RtmpData, ()> {
        let mut cursor = AMFDecodingCursor::new(data);

        let tag_amf = AMF0Value::read(&mut cursor, data)?;
        let tag = tag_amf.get_string();

        let mut d = RtmpData::new(tag.to_string());

        let arg_list_res = RTMP_DATA_CODES.get(tag);

        if let Some(arg_list) = arg_list_res {
            let mut i: usize = 0;

            while i < arg_list.len() && !cursor.ended() {
                let val = AMF0Value::read(&mut cursor, data)?;

                d.set_argument(arg_list[i].clone(), val);

                i += 1;
            }
        }

        Ok(d)
    }
}
