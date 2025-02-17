// AMF parsers and serializers

mod amf0;
mod cursor;

pub use amf0::*;
pub use cursor::*;

// Tests

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;

    fn matches_properties(
        p1: &HashMap<String, AMF0Value>,
        p2: &HashMap<String, AMF0Value>,
    ) -> bool {
        for (k, v1) in p1 {
            let v2_opt = p2.get(k);

            match v2_opt {
                Some(v2) => {
                    if !amf_equals(v1, v2) {
                        return false;
                    }
                }
                None => {
                    return false;
                }
            }
        }

        for (k, v1) in p2 {
            let v2_opt = p1.get(k);

            match v2_opt {
                Some(v2) => {
                    if !amf_equals(v1, v2) {
                        return false;
                    }
                }
                None => {
                    return false;
                }
            }
        }

        true
    }

    fn matches_items(a1: &[AMF0Value], a2: &[AMF0Value]) -> bool {
        if a1.len() != a2.len()  {
            return false
        }

        for i in 0..a1.len() {
            if !amf_equals(&a1[i], &a2[i]) {
                return false;
            }
        }

        true
    }

    fn amf_equals(v1: &AMF0Value, v2: &AMF0Value) -> bool {
        match v1 {
            AMF0Value::Number { value } => {
                if let AMF0Value::Number { value: value2 } = v2 {
                    value == value2
                } else {
                    false
                }
            }
            AMF0Value::Bool { value } => {
                if let AMF0Value::Bool { value: value2 } = v2 {
                    value == value2
                } else {
                    false
                }
            }
            AMF0Value::String { value } => {
                if let AMF0Value::String { value: value2 } = v2 {
                    value == value2
                } else {
                    false
                }
            }
            AMF0Value::Object { properties } => {
                if let AMF0Value::Object {
                    properties: properties2,
                } = v2
                {
                    matches_properties(properties, properties2)
                } else {
                    false
                }
            }
            AMF0Value::Null => matches!(v2, AMF0Value::Null),
            AMF0Value::Undefined => matches!(v2, AMF0Value::Undefined),
            AMF0Value::Ref { addr } => {
                if let AMF0Value::Ref { addr: addr2 } = v2 {
                    addr == addr2
                } else {
                    false
                }
            }
            AMF0Value::Array { items } => {
                if let AMF0Value::Array { items: items2 } = v2 {
                    matches_properties(items, items2)
                } else {
                    false
                }
            }
            AMF0Value::StrictArray { items } => {
                if let AMF0Value::StrictArray { items: items2 } = v2 {
                    matches_items(items, items2)
                } else {
                    false
                }
            },
            AMF0Value::Date { timestamp } => {
                if let AMF0Value::Date {
                    timestamp: timestamp2,
                } = v2
                {
                    timestamp == timestamp2
                } else {
                    false
                }
            }
            AMF0Value::LongString { value } => {
                if let AMF0Value::LongString { value: value2 } = v2 {
                    value == value2
                } else {
                    false
                }
            }
            AMF0Value::XmlDocument { content } => {
                if let AMF0Value::XmlDocument { content: content2 } = v2 {
                    content == content2
                } else {
                    false
                }
            }
            AMF0Value::TypedObject {
                type_name,
                properties,
            } => {
                if let AMF0Value::TypedObject {
                    type_name: type_name2,
                    properties: properties2,
                } = v2
                {
                    type_name == type_name2 && matches_properties(properties, properties2)
                } else {
                    false
                }
            }
        }
    }

    fn test_encode_decode(v: &AMF0Value) -> bool {
        let encoded = v.encode();
        let mut cursor = AMFDecodingCursor::new(&encoded);
        let decoded = AMF0Value::read(&mut cursor, &encoded);

        match decoded {
            Ok(v2) => {
                if amf_equals(v, &v2) {
                    true
                } else {
                    panic!("No match: \n{}\n{}", v.to_debug_string(""), v2.to_debug_string(""));
                }
            },
            Err(_) => {
                panic!("Error decoding value: \n{}", v.to_debug_string(""));
            },
        }
    }

    #[test]
    fn test_amf_encode_decode() {
        assert!(test_encode_decode(&AMF0Value::Undefined));
        assert!(test_encode_decode(&AMF0Value::Null));

        assert!(test_encode_decode(&AMF0Value::Number { value: 0.0 }));
        assert!(test_encode_decode(&AMF0Value::Number { value: 100.0 }));
        assert!(test_encode_decode(&AMF0Value::Number { value: -100.0 }));
        assert!(test_encode_decode(&AMF0Value::Number { value: 100.5 }));
        assert!(test_encode_decode(&AMF0Value::Number { value: -100.5 }));

        assert!(test_encode_decode(&AMF0Value::Bool { value: true }));
        assert!(test_encode_decode(&AMF0Value::Bool { value: false }));

        assert!(test_encode_decode(&AMF0Value::String { value: "".to_string() }));
        assert!(test_encode_decode(&AMF0Value::String { value: "test".to_string() }));

        assert!(test_encode_decode(&AMF0Value::LongString { value: "".to_string() }));
        assert!(test_encode_decode(&AMF0Value::LongString { value: "test".to_string() }));

        assert!(test_encode_decode(&AMF0Value::XmlDocument { content: "".to_string() }));
        assert!(test_encode_decode(&AMF0Value::XmlDocument { content: "test".to_string() }));

        assert!(test_encode_decode(&AMF0Value::Ref { addr: 0 }));
        assert!(test_encode_decode(&AMF0Value::Ref { addr: 100 }));
        assert!(test_encode_decode(&AMF0Value::Ref { addr: u16::MAX }));

        assert!(test_encode_decode(&AMF0Value::Date { timestamp: Utc::now().timestamp() as f64 }));

        // Test objects

        let mut props: HashMap<String, AMF0Value> = HashMap::new();

        props.insert("test_prop_1".to_string(), AMF0Value::Null);
        props.insert("test_prop_2".to_string(), AMF0Value::Number { value: 1.5 });
        props.insert("test_prop_2".to_string(), AMF0Value::String { value: "test_str".to_string() });

        assert!(test_encode_decode(&AMF0Value::Object { properties: props.clone() }));
        assert!(test_encode_decode(&AMF0Value::Array { items: props.clone() }));
        assert!(test_encode_decode(&AMF0Value::TypedObject { type_name: "test.type.object".to_string(), properties: props}));

        // Test strict array

        let items: Vec<AMF0Value> = vec![AMF0Value::Null, AMF0Value::Number { value: 1.5 }, AMF0Value::String { value: "test_str".to_string() }];

        assert!(test_encode_decode(&AMF0Value::StrictArray { items: items }));
    }
}
