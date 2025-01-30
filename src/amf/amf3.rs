// AMF3 value

use byteorder::{BigEndian, ByteOrder};

use super::AMFDecodingCursor;

const AMF3_TYPE_UNDEFINED: u8 = 0x00;
const AMF3_TYPE_NULL: u8 = 0x01;
const AMF3_TYPE_FALSE: u8 = 0x02;
const AMF3_TYPE_TRUE: u8 = 0x03;
const AMF3_TYPE_INTEGER: u8 = 0x04;
const AMF3_TYPE_DOUBLE: u8 = 0x05;
const AMF3_TYPE_STRING: u8 = 0x06;
const AMF3_TYPE_XML_DOC: u8 = 0x07;
const AMF3_TYPE_DATE: u8 = 0x08;
const AMF3_TYPE_ARRAY: u8 = 0x09;
const AMF3_TYPE_OBJECT: u8 = 0x0A;
const AMF3_TYPE_XML: u8 = 0x0B;
const AMF3_TYPE_BYTE_ARRAY: u8 = 0x0C;

/// AMF3 compatible value
#[derive(Clone)]
pub enum AMF3Value {
    Undefined,
    Null,
    False,
    True,
    Integer { value: i32 },
    Double { value: f64 },
    String { value: String },
    XmlDocument { content: String },
    Date { timestamp: f64 },
    Array,
    Object,
    Xml { value: String },
    ByteArray { value: Vec<u8> },
}

impl AMF3Value {
    /// Obtains a string representation of the value
    /// Used for debug logging purposes
    pub fn to_debug_string(&self, _tabs: &str) -> String {
        match self {
            AMF3Value::Undefined => "Undefined".to_string(),
            AMF3Value::Null => "Null".to_string(),
            AMF3Value::False => "False".to_string(),
            AMF3Value::True => "True".to_string(),
            AMF3Value::Integer { value } => format!("Integer({})", value),
            AMF3Value::Double { value } => format!("Double({})", value),
            AMF3Value::String { value } => format!("'{}'", value),
            AMF3Value::XmlDocument { content } => format!("XML_DOC'{}'", content),
            AMF3Value::Date { timestamp } => format!("DATE({})", timestamp),
            AMF3Value::Array => "Array(Unsupported)".to_string(),
            AMF3Value::Object => "Object(Unsupported)".to_string(),
            AMF3Value::Xml { value } => format!("XML'{}'", value),
            AMF3Value::ByteArray { value } => format!("Bytes({})", hex::encode(value)),
        }
    }

    // Value check functions:

    /// Turns the ANF3 value into a boolean
    pub fn get_bool(&self) -> bool {
        match self {
            AMF3Value::True => true,
            _ => false,
        }
    }

    /// Returns true if the value is undefined
    pub fn is_undefined(&self) -> bool {
        match self {
            AMF3Value::Undefined => true,
            _ => false,
        }
    }

    /// Returns true if the value is null
    pub fn is_null(&self) -> bool {
        match self {
            AMF3Value::Null => true,
            _ => false,
        }
    }

    /// Returns the value as integer
    pub fn get_integer(&self) -> i64 {
        match self {
            AMF3Value::Integer { value } => *value as i64,
            AMF3Value::Double { value } => *value as i64,
            _ => 0,
        }
    }

    /// Returns the value as float
    pub fn get_float(&self) -> f64 {
        match self {
            AMF3Value::Integer { value } => *value as f64,
            AMF3Value::Double { value } => *value,
            _ => 0.0,
        }
    }

    /// Returns the value as string
    pub fn get_string(&self) -> &str {
        match self {
            AMF3Value::String { value } => value.as_str(),
            AMF3Value::XmlDocument { content } => content.as_str(),
            AMF3Value::Xml { value } => value.as_str(),
            _ => "",
        }
    }

    /// Returns the value as string
    pub fn get_byte_array(&self) -> Option<&Vec<u8>> {
        match self {
            AMF3Value::ByteArray { value } => Some(&value),
            _ => None,
        }
    }

    // Encoding functions:

    /// Encodes value into bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            AMF3Value::Undefined => vec![AMF3_TYPE_UNDEFINED],
            AMF3Value::Null => vec![AMF3_TYPE_NULL],
            AMF3Value::False => vec![AMF3_TYPE_FALSE],
            AMF3Value::True => vec![AMF3_TYPE_TRUE],
            AMF3Value::Integer { value } => {
                let mut buf = vec![AMF3_TYPE_INTEGER];
                buf.extend(Self::encode_integer(*value));
                buf
            }
            AMF3Value::Double { value } => {
                let mut buf = vec![AMF3_TYPE_DOUBLE];
                buf.extend(Self::encode_double(*value));
                buf
            }
            AMF3Value::String { value } => {
                let mut buf = vec![AMF3_TYPE_STRING];
                buf.extend(Self::encode_string(value));
                buf
            }
            AMF3Value::XmlDocument { content } => {
                let mut buf = vec![AMF3_TYPE_XML_DOC];
                buf.extend(Self::encode_string(content));
                buf
            }
            AMF3Value::Date { timestamp } => {
                let mut buf = vec![AMF3_TYPE_DATE];
                buf.extend(Self::encode_date(*timestamp));
                buf
            }
            AMF3Value::Array => vec![AMF3_TYPE_ARRAY],
            AMF3Value::Object => vec![AMF3_TYPE_OBJECT],
            AMF3Value::Xml { value } => {
                let mut buf = vec![AMF3_TYPE_XML];
                buf.extend(Self::encode_string(value));
                buf
            }
            AMF3Value::ByteArray { value } => {
                let mut buf = vec![AMF3_TYPE_BYTE_ARRAY];
                buf.extend(Self::encode_byte_array(value));
                buf
            }
        }
    }

    /// Encodes unsigned integer with the format UI29
    pub fn encode_ui29(num: u32) -> Vec<u8> {
        if num < 0x80 {
            vec![num as u8]
        } else if num < 0x4000 {
            vec![(num & 0x7F) as u8, ((num >> 7) | 0x80) as u8]
        } else if num < 0x200000 {
            vec![
                (num & 0x7F) as u8,
                ((num >> 7) & 0x7F) as u8,
                ((num >> 14) | 0x80) as u8,
            ]
        } else {
            vec![
                (num & 0xFF) as u8,
                ((num >> 8) & 0x7F) as u8,
                ((num >> 15) | 0x7F) as u8,
                ((num >> 22) | 0x7F) as u8,
            ]
        }
    }

    /// Encodes string value
    pub fn encode_string(val: &str) -> Vec<u8> {
        let str_bytes = val.as_bytes();
        let mut buf = Self::encode_ui29((str_bytes.len() as u32) << 1);

        buf.extend(str_bytes);

        buf
    }

    /// Encodes integer value
    pub fn encode_integer(i: i32) -> Vec<u8> {
        Self::encode_ui29((i as u32) & 0x3FFFFFFF)
    }

    /// Encodes double value
    pub fn encode_double(d: f64) -> Vec<u8> {
        let mut buf = vec![0; 8];
        BigEndian::write_f64(&mut buf, d);
        buf
    }

    /// Encodes date
    pub fn encode_date(ts: f64) -> Vec<u8> {
        let mut buf = Self::encode_ui29(1);
        buf.extend(Self::encode_double(ts));
        buf
    }

    /// Encodes byte array
    pub fn encode_byte_array(bytes: &[u8]) -> Vec<u8> {
        let mut buf = Self::encode_ui29((bytes.len() as u32) << 1);
        buf.extend(bytes);
        buf
    }

    // Decoding functions:

    /// Reads and decodes an integer in UI29 format
    pub fn decode_ui29(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<u32, ()> {
        let mut val: u32 = 0;
        let mut len: u32 = 0;
        let mut ended: bool = false;
        let mut b: u8 = 0x00;

        while !ended {
            b = cursor.read_byte(buffer)?;

            len += 1;

            val = (val << 7).wrapping_add((b & 0x7F) as u32);

            ended = !(len < 5 || b > 0x7F);
        }

        if len == 5 {
            val = val | (b as u32);
        }

        Ok(val)
    }

    /// Reads an instance of AMF3Value from a buffer
    pub fn read(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<AMF3Value, ()> {
        let amf3_type = cursor.read_byte(buffer)?;

        match amf3_type {
            AMF3_TYPE_NULL => Ok(AMF3Value::Null),
            AMF3_TYPE_FALSE => Ok(AMF3Value::False),
            AMF3_TYPE_TRUE => Ok(AMF3Value::True),
            AMF3_TYPE_INTEGER => Ok(AMF3Value::Integer {
                value: Self::decode_ui29(cursor, buffer)? as i32,
            }),
            AMF3_TYPE_DOUBLE => Ok(AMF3Value::Double {
                value: Self::read_double(cursor, buffer)?,
            }),
            AMF3_TYPE_DATE => Ok(AMF3Value::Date {
                timestamp: Self::read_date(cursor, buffer)?,
            }),
            AMF3_TYPE_STRING => Ok(AMF3Value::String {
                value: Self::read_string(cursor, buffer)?,
            }),
            AMF3_TYPE_XML => Ok(AMF3Value::Xml {
                value: Self::read_string(cursor, buffer)?,
            }),
            AMF3_TYPE_XML_DOC => Ok(AMF3Value::XmlDocument {
                content: Self::read_string(cursor, buffer)?,
            }),
            AMF3_TYPE_BYTE_ARRAY => Ok(AMF3Value::ByteArray {
                value: Self::read_byte_array(cursor, buffer)?,
            }),
            AMF3_TYPE_ARRAY => Ok(AMF3Value::Array),
            AMF3_TYPE_OBJECT => Ok(AMF3Value::Object),
            _ => Ok(AMF3Value::Undefined),
        }
    }

    /// Reads double in AMF3 format from buffer
    pub fn read_double(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<f64, ()> {
        let buf = cursor.read(buffer, 8)?;

        if buf.len() < 8 {
            return Err(());
        }

        Ok(BigEndian::read_f64(buf))
    }

    /// Reads date in AMF3 format from buffer
    pub fn read_date(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<f64, ()> {
        Self::decode_ui29(cursor, buffer)?; // Skip prefix
        Self::read_double(cursor, buffer)
    }

    /// Reads string in AMF3 format from buffer
    pub fn read_string(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<String, ()> {
        let l = Self::decode_ui29(cursor, buffer)?;

        let str_bytes = cursor.read(buffer, l as usize)?;

        let str_res = String::from_utf8(str_bytes.to_vec());

        match str_res {
            Ok(s) => Ok(s),
            Err(_) => Err(()),
        }
    }

    /// Reads byte array in AMF3 format from buffer
    pub fn read_byte_array(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<Vec<u8>, ()> {
        let l = Self::decode_ui29(cursor, buffer)?;

        let bytes = cursor.read(buffer, l as usize)?;

        Ok(bytes.to_vec())
    }
}
