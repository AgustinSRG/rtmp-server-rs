// AMF0 value

use byteorder::{BigEndian, ByteOrder};
use std::collections::HashMap;

use super::AMFDecodingCursor;

const AMF0_TYPE_NUMBER: u8 = 0x00;
const AMF0_TYPE_BOOL: u8 = 0x01;
const AMF0_TYPE_STRING: u8 = 0x02;
const AMF0_TYPE_OBJECT: u8 = 0x03;
const AMF0_TYPE_NULL: u8 = 0x05;
const AMF0_TYPE_UNDEFINED: u8 = 0x06;
const AMF0_TYPE_REF: u8 = 0x07;
const AMF0_TYPE_ARRAY: u8 = 0x08;
const AMF0_TYPE_STRICT_ARRAY: u8 = 0x0A;
const AMF0_TYPE_DATE: u8 = 0x0B;
const AMF0_TYPE_LONG_STRING: u8 = 0x0C;
const AMF0_TYPE_XML_DOC: u8 = 0x0F;
const AMF0_TYPE_TYPED_OBJ: u8 = 0x10;

const AMF0_OBJECT_TERM_CODE: u8 = 0x09;

/// AMF0 compatible value
#[derive(Clone)]
pub enum AMF0Value {
    Number {
        value: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    Object {
        properties: HashMap<String, AMF0Value>,
    },
    Null,
    Undefined,
    Ref {
        addr: u16,
    },
    Array {
        items: HashMap<String, AMF0Value>,
    },
    StrictArray {
        items: Vec<AMF0Value>,
    },
    Date {
        timestamp: f64,
    },
    LongString {
        value: String,
    },
    XmlDocument {
        content: String,
    },
    TypedObject {
        type_name: String,
        properties: HashMap<String, AMF0Value>,
    },
}

impl AMF0Value {
    /// Obtains a string representation of the value
    /// Used for debug logging purposes
    pub fn to_debug_string(&self, tabs: &str) -> String {
        match self {
            AMF0Value::Number { value } => {
                format!("{}", value)
            }
            AMF0Value::Bool { value } => {
                if *value {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            AMF0Value::String { value } => {
                format!("'{}'", value)
            }
            AMF0Value::Object { properties } => {
                let mut res = "{\n".to_string();

                for (key, value) in properties.iter() {
                    res.push_str(tabs);
                    res.push_str("    '");
                    res.push_str(key);
                    res.push_str("' = ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push('\n');
                }

                res.push_str(tabs);
                res.push('}');

                res
            }
            AMF0Value::Null => "NULL".to_string(),
            AMF0Value::Undefined => "UNDEFINED".to_string(),
            AMF0Value::Ref { addr } => {
                format!("REF#{}", addr)
            }
            AMF0Value::Array { items } => {
                let mut res = "ARRAY [\n".to_string();

                for (key, value) in items.iter() {
                    res.push_str(tabs);
                    res.push_str("    '");
                    res.push_str(key);
                    res.push_str("' = ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push('\n');
                }

                res.push_str(tabs);
                res.push(']');

                res
            }
            AMF0Value::StrictArray { items } => {
                let mut res = "STRICT_ARRAY [\n".to_string();

                for value in items.iter() {
                    res.push_str(tabs);
                    res.push_str("    ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push('\n');
                }

                res.push_str(tabs);
                res.push(']');

                res
            }
            AMF0Value::Date { timestamp } => {
                format!("DATE({})", timestamp)
            }
            AMF0Value::LongString { value } => {
                format!("L'{}'", value)
            }
            AMF0Value::XmlDocument { content } => {
                format!("XML'{}'", content)
            }
            AMF0Value::TypedObject {
                type_name,
                properties,
            } => {
                let mut res = format!("{} {}\n", type_name, "{");

                for (key, value) in properties.iter() {
                    res.push_str(tabs);
                    res.push_str("    '");
                    res.push_str(key);
                    res.push_str("' = ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push('\n');
                }

                res.push_str(tabs);
                res.push('}');

                res
            }
        }
    }

    // Value check functions:

    /// Returns true if the value is undefined
    pub fn is_undefined(&self) -> bool {
        matches!(self, AMF0Value::Undefined)
    }

    /// Returns the value as boolean
    pub fn get_bool(&self) -> bool {
        match self {
            AMF0Value::Bool { value } => *value,
            AMF0Value::Number { value } => *value != 0.0,
            _ => false,
        }
    }

    /// Returns the value as integer
    pub fn get_integer(&self) -> i64 {
        match self {
            AMF0Value::Number { value } => *value as i64,
            AMF0Value::Ref { addr } => *addr as i64,
            AMF0Value::Date { timestamp } => *timestamp as i64,
            _ => 0,
        }
    }

    /// Returns the value as float
    pub fn get_string(&self) -> &str {
        match self {
            AMF0Value::String { value } => value.as_str(),
            AMF0Value::LongString { value } => value.as_str(),
            AMF0Value::XmlDocument { content } => content.as_str(),
            _ => "",
        }
    }

    /// Returns the value as object (HashMap)
    pub fn get_object(&self) -> Option<&HashMap<String, AMF0Value>> {
        match self {
            AMF0Value::Object { properties } => Some(properties),
            AMF0Value::Array { items } => Some(items),
            AMF0Value::TypedObject {
                type_name: _,
                properties,
            } => Some(properties),
            _ => None,
        }
    }

    /// Gets the value of a property (for objects)
    pub fn get_object_property(&self, property_name: &str) -> Option<&AMF0Value> {
        let obj = self.get_object();

        match obj {
            Some(o) => o.get(property_name),
            None => None,
        }
    }

    // Encoding functions:

    /// Encodes value into bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            AMF0Value::Number { value } => {
                let mut buf = vec![AMF0_TYPE_NUMBER];
                buf.extend(Self::encode_number(*value));
                buf
            }
            AMF0Value::Bool { value } => {
                let mut buf = vec![AMF0_TYPE_BOOL];
                buf.extend(Self::encode_bool(*value));
                buf
            }
            AMF0Value::String { value } => {
                let mut buf = vec![AMF0_TYPE_STRING];
                buf.extend(Self::encode_string(value));
                buf
            }
            AMF0Value::Object { properties } => {
                let mut buf = vec![AMF0_TYPE_OBJECT];
                buf.extend(Self::encode_object(properties));
                buf
            }
            AMF0Value::Null => vec![AMF0_TYPE_NULL],
            AMF0Value::Undefined => vec![AMF0_TYPE_UNDEFINED],
            AMF0Value::Ref { addr } => {
                let mut buf = vec![AMF0_TYPE_REF];
                buf.extend(Self::encode_ref(*addr));
                buf
            }
            AMF0Value::Array { items } => {
                let mut buf = vec![AMF0_TYPE_ARRAY];
                buf.extend(Self::encode_array(items));
                buf
            }
            AMF0Value::StrictArray { items } => {
                let mut buf = vec![AMF0_TYPE_STRICT_ARRAY];
                buf.extend(Self::encode_strict_array(items));
                buf
            }
            AMF0Value::Date { timestamp } => {
                let mut buf = vec![AMF0_TYPE_DATE];
                buf.extend(Self::encode_date(*timestamp));
                buf
            }
            AMF0Value::LongString { value } => {
                let mut buf = vec![AMF0_TYPE_LONG_STRING];
                buf.extend(Self::encode_long_string(value));
                buf
            }
            AMF0Value::XmlDocument { content } => {
                let mut buf = vec![AMF0_TYPE_XML_DOC];
                buf.extend(Self::encode_string(content));
                buf
            }
            AMF0Value::TypedObject {
                type_name,
                properties,
            } => {
                let mut buf = vec![AMF0_TYPE_TYPED_OBJ];
                buf.extend(Self::encode_typed_object(type_name, properties));
                buf
            }
        }
    }

    /// Encodes number value
    pub fn encode_number(num: f64) -> Vec<u8> {
        let mut buf = vec![0; 8];
        BigEndian::write_f64(&mut buf, num);
        buf
    }

    /// Encodes boolean value
    pub fn encode_bool(b: bool) -> Vec<u8> {
        if b {
            vec![0x01]
        } else {
            vec![0x00]
        }
    }

    /// Encodes date value
    pub fn encode_date(ts: f64) -> Vec<u8> {
        let mut buf = vec![0x00, 0x00];
        buf.extend(Self::encode_number(ts));
        buf
    }

    /// Encodes string value
    pub fn encode_string(s: &str) -> Vec<u8> {
        let str_bytes = s.bytes();
        let mut buf = vec![0x00; 2];
        BigEndian::write_u16(&mut buf, str_bytes.len() as u16);
        buf.extend(str_bytes);
        buf
    }

    /// Encodes long string value
    pub fn encode_long_string(s: &str) -> Vec<u8> {
        let str_bytes = s.bytes();
        let mut buf = vec![0x00; 4];
        BigEndian::write_u32(&mut buf, str_bytes.len() as u32);
        buf.extend(str_bytes);
        buf
    }

    /// Encodes object value
    pub fn encode_object(o: &HashMap<String, AMF0Value>) -> Vec<u8> {
        let mut buf = Vec::new();

        let mut keys: Vec<&str> = Vec::with_capacity(o.len());

        for key in o.keys() {
            keys.push(key);
        }

        keys.sort();

        for key in keys {
            buf.extend(Self::encode_string(key));
            let value = o.get(key).unwrap();
            buf.extend(value.encode());
        }

        buf.extend(Self::encode_string(""));
        buf.extend(vec![AMF0_OBJECT_TERM_CODE]);

        buf
    }

    /// Encodes array value
    pub fn encode_array(arr: &HashMap<String, AMF0Value>) -> Vec<u8> {
        let mut buf = vec![0; 4];
        BigEndian::write_u32(&mut buf, arr.len() as u32);
        buf.extend(Self::encode_object(arr));
        buf
    }

    /// Encodes strict array value
    pub fn encode_strict_array(arr: &Vec<AMF0Value>) -> Vec<u8> {
        let mut buf = vec![0; 4];
        BigEndian::write_u32(&mut buf, arr.len() as u32);

        for item in arr {
            buf.extend(item.encode());
        }

        buf
    }

    /// Encodes reference value
    pub fn encode_ref(index: u16) -> Vec<u8> {
        let mut buf = vec![0x00; 2];
        BigEndian::write_u16(&mut buf, index);
        buf
    }

    /// Encodes typed object value
    pub fn encode_typed_object(type_name: &str, o: &HashMap<String, AMF0Value>) -> Vec<u8> {
        let mut buf = Self::encode_string(type_name);
        buf.extend(Self::encode_object(o));
        buf
    }

    // Deciding functions:

    /// Reads AMF0 value from buffer
    pub fn read(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<AMF0Value, ()> {
        let amf0_type = cursor.read_byte(buffer)?;

        match amf0_type {
            AMF0_TYPE_UNDEFINED => Ok(AMF0Value::Undefined),
            AMF0_TYPE_NULL => Ok(AMF0Value::Null),
            AMF0_TYPE_NUMBER => Ok(AMF0Value::Number {
                value: Self::read_number(cursor, buffer)?,
            }),
            AMF0_TYPE_BOOL => Ok(AMF0Value::Bool {
                value: Self::read_bool(cursor, buffer)?,
            }),
            AMF0_TYPE_DATE => Ok(AMF0Value::Date {
                timestamp: Self::read_date(cursor, buffer)?,
            }),
            AMF0_TYPE_STRING => Ok(AMF0Value::String {
                value: Self::read_string(cursor, buffer)?,
            }),
            AMF0_TYPE_XML_DOC => Ok(AMF0Value::XmlDocument {
                content: Self::read_string(cursor, buffer)?,
            }),
            AMF0_TYPE_LONG_STRING => Ok(AMF0Value::LongString {
                value: Self::read_long_string(cursor, buffer)?,
            }),
            AMF0_TYPE_OBJECT => Ok(AMF0Value::Object {
                properties: Self::read_object(cursor, buffer)?,
            }),
            AMF0_TYPE_TYPED_OBJ => {
                let (type_name, properties) = Self::read_typed_object(cursor, buffer)?;

                Ok(AMF0Value::TypedObject {
                    type_name,
                    properties,
                })
            }
            AMF0_TYPE_REF => Ok(AMF0Value::Ref {
                addr: Self::read_u16_be(cursor, buffer)?,
            }),
            AMF0_TYPE_ARRAY => Ok(AMF0Value::Array {
                items: Self::read_array(cursor, buffer)?,
            }),
            AMF0_TYPE_STRICT_ARRAY => Ok(AMF0Value::StrictArray {
                items: Self::read_strict_array(cursor, buffer)?,
            }),
            _ => Ok(AMF0Value::Undefined),
        }
    }

    /// Reads number from buffer
    pub fn read_number(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<f64, ()> {
        let buf = cursor.read(buffer, 8)?;

        if buf.len() < 8 {
            return Err(());
        }

        Ok(BigEndian::read_f64(buf))
    }

    /// Reads number from buffer
    pub fn read_date(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<f64, ()> {
        cursor.skip(2)?; // Skip prefix
        Self::read_number(cursor, buffer)
    }

    /// Reads boolean from buffer
    pub fn read_bool(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<bool, ()> {
        let b = cursor.read_byte(buffer)?;
        Ok(b != 0x00)
    }

    /// Reads u16 (big endian)
    pub fn read_u16_be(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<u16, ()> {
        let buf = cursor.read(buffer, 2)?;

        if buf.len() < 2 {
            return Err(());
        }

        Ok(BigEndian::read_u16(buf))
    }

    /// Reads string from buffer
    pub fn read_string(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<String, ()> {
        let l = Self::read_u16_be(cursor, buffer)?;

        let str_bytes = cursor.read(buffer, l as usize)?;

        let str_res = String::from_utf8(str_bytes.to_vec());

        match str_res {
            Ok(s) => Ok(s),
            Err(_) => Err(()),
        }
    }

    /// Reads u32 (big endian)
    pub fn read_u32_be(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<u32, ()> {
        let buf = cursor.read(buffer, 4)?;

        if buf.len() < 4 {
            return Err(());
        }

        Ok(BigEndian::read_u32(buf))
    }

    /// Reads long string from buffer
    pub fn read_long_string(cursor: &mut AMFDecodingCursor, buffer: &[u8]) -> Result<String, ()> {
        let l = Self::read_u32_be(cursor, buffer)?;

        let str_bytes = cursor.read(buffer, l as usize)?;

        let str_res = String::from_utf8(str_bytes.to_vec());

        match str_res {
            Ok(s) => Ok(s),
            Err(_) => Err(()),
        }
    }

    /// Reads object from buffer
    pub fn read_object(
        cursor: &mut AMFDecodingCursor,
        buffer: &[u8],
    ) -> Result<HashMap<String, AMF0Value>, ()> {
        let mut o: HashMap<String, AMF0Value> = HashMap::new();

        while !cursor.ended() {
            let prop_name = Self::read_string(cursor, buffer)?;

            let next_byte = cursor.look_byte(buffer)?;

            if next_byte == AMF0_OBJECT_TERM_CODE {
                break;
            }

            let prop_value = Self::read(cursor, buffer)?;

            o.insert(prop_name, prop_value);
        }

        Ok(o)
    }

    /// Reads array from buffer
    pub fn read_array(
        cursor: &mut AMFDecodingCursor,
        buffer: &[u8],
    ) -> Result<HashMap<String, AMF0Value>, ()> {
        cursor.skip(4)?;
        Self::read_object(cursor, buffer)
    }

    /// Reads strict array from buffer
    pub fn read_strict_array(
        cursor: &mut AMFDecodingCursor,
        buffer: &[u8],
    ) -> Result<Vec<AMF0Value>, ()> {
        let mut arr: Vec<AMF0Value> = Vec::new();

        let mut l = Self::read_u32_be(cursor, buffer)?;

        while l > 0 {
            let item = Self::read(cursor, buffer)?;

            arr.push(item);

            l -= 1;
        }

        Ok(arr)
    }

    /// Reads typed object from buffer
    pub fn read_typed_object(
        cursor: &mut AMFDecodingCursor,
        buffer: &[u8],
    ) -> Result<(String, HashMap<String, AMF0Value>), ()> {
        let type_name = Self::read_string(cursor, buffer)?;
        let o = Self::read_object(cursor, buffer)?;
        Ok((type_name, o))
    }
}
