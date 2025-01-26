// AMF0 value

use std::collections::HashMap;

use super::AMF3Value;

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
const AMF0_TYPE_SWITCH_AMF3: u8 = 0x11;

const AMF0_OBJECT_TERM_CODE: u8 = 0x09;

/// AMF0 compatible value
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
        addr: i64,
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
    SwitchAmf3 {
        value: AMF3Value,
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

                for (key, value) in properties.into_iter() {
                    res.push_str(tabs);
                    res.push_str("    '");
                    res.push_str(key);
                    res.push_str("' = ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push_str("\n");
                }

                res.push_str(tabs);
                res.push_str("}");

                res
            }
            AMF0Value::Null => "NULL".to_string(),
            AMF0Value::Undefined => "UNDEFINED".to_string(),
            AMF0Value::Ref { addr } => {
                format!("REF#{}", addr)
            }
            AMF0Value::Array { items } => {
                let mut res = "ARRAY [\n".to_string();

                for (key, value) in items.into_iter() {
                    res.push_str(tabs);
                    res.push_str("    '");
                    res.push_str(key);
                    res.push_str("' = ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push_str("\n");
                }

                res.push_str(tabs);
                res.push_str("]");

                res
            }
            AMF0Value::StrictArray { items } => {
                let mut res = "STRICT_ARRAY [\n".to_string();

                for value in items.into_iter() {
                    res.push_str(tabs);
                    res.push_str("    ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push_str("\n");
                }

                res.push_str(tabs);
                res.push_str("]");

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

                for (key, value) in properties.into_iter() {
                    res.push_str(tabs);
                    res.push_str("    '");
                    res.push_str(key);
                    res.push_str("' = ");
                    res.push_str(&value.to_debug_string(&format!("{}    ", tabs)));
                    res.push_str("\n");
                }

                res.push_str(tabs);
                res.push_str("}");

                res
            }
            AMF0Value::SwitchAmf3 { value } => format!("AMF3({})", value.to_debug_string(tabs)),
        }
    }

    /// Returns true if the value is AMF3
    pub fn is_amf3(&self) -> bool {
        match self {
            AMF0Value::SwitchAmf3 { value: _ } => true,
            _ => false,
        }
    }

    /// Returns true if the value is undefined
    pub fn is_undefined(&self) -> bool {
        match self {
            AMF0Value::Undefined => true,
            AMF0Value::SwitchAmf3 { value } => value.is_undefined(),
            _ => false,
        }
    }

    /// Returns true if the value is null
    pub fn is_null(&self) -> bool {
        match self {
            AMF0Value::Null => true,
            AMF0Value::SwitchAmf3 { value } => value.is_null(),
            _ => false,
        }
    }

    /// Returns the value as boolean
    pub fn get_bool(&self) -> bool {
        match self {
            AMF0Value::Bool { value } => *value,
            AMF0Value::Number { value } => *value != 0.0,
            AMF0Value::SwitchAmf3 { value } => value.get_bool(),
            _ => false,
        }
    }

    /// Returns the value as integer
    pub fn get_integer(&self) -> i64 {
        match self {
            AMF0Value::Number { value } => *value as i64,
            AMF0Value::Ref { addr } => *addr,
            AMF0Value::Date { timestamp } => *timestamp as i64,
            AMF0Value::SwitchAmf3 { value } => value.get_integer(),
            _ => 0,
        }
    }

    /// Returns the value as float
    pub fn get_float(&self) -> f64 {
        match self {
            AMF0Value::Number { value } => *value,
            AMF0Value::Ref { addr } => *addr as f64,
            AMF0Value::Date { timestamp } => *timestamp,
            AMF0Value::SwitchAmf3 { value } => value.get_float(),
            _ => 0.0,
        }
    }

    /// Returns the value as float
    pub fn get_string(&self) -> &str {
        match self {
            AMF0Value::String { value } => value.as_str(),
            AMF0Value::LongString { value } => value.as_str(),
            AMF0Value::XmlDocument { content } => content.as_str(),
            AMF0Value::SwitchAmf3 { value } => value.get_string(),
            _ => "",
        }
    }

    /// Returns the value as byte array
    pub fn get_byte_array(&self) -> Option<&Vec<u8>> {
        match self {
            AMF0Value::SwitchAmf3 { value } => value.get_byte_array(),
            _ => None,
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

    /// Returns the value as array (Vec)
    pub fn get_array(&self) -> Option<&Vec<AMF0Value>> {
        match self {
            AMF0Value::StrictArray { items } => Some(items),
            _ => None,
        }
    }

    /// Gets an element of the array, given its index
    pub fn get_array_element(&self, index: usize) -> Option<&AMF0Value> {
        let arr = self.get_array();

        match arr {
            Some(a) => a.get(index),
            None => None,
        }
    }
}
