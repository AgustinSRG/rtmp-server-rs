// AMF3 value

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
    pub fn to_debug_string(&self, tabs: &str) -> String {
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


}
