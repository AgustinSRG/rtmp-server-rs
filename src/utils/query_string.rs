// Query string utilities

use std::collections::HashMap;

/// Parses query string (does not parse parameters)
/// Used in order to parse RTMP play parameters
///
/// # Arguments
///
/// * `query_string` - The query string part of the URI
///
/// # Return value
///
/// A map with all the key-value pairs the query string contains
pub fn parse_query_string_simple(query_string: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    if !query_string.is_empty() {
        let parts = query_string.split("&");

        for part in parts {
            let key_val: Vec<&str> = part.split("=").collect();

            if key_val.len() == 2 {
                result.insert(key_val[0].to_string(), key_val[1].to_string());
            }
        }
    }

    result
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_string_simple() {
        let params_1 = parse_query_string_simple("");

        assert!(params_1.is_empty());

        let params_2 = parse_query_string_simple("cache=clear");

        assert!(!params_2.is_empty());
        assert_eq!(params_2.get("cache").unwrap(), "clear");

        let params_3 = parse_query_string_simple("cache=clear&opt=1");

        assert!(!params_3.is_empty());
        assert_eq!(params_3.get("cache").unwrap(), "clear");
        assert_eq!(params_3.get("opt").unwrap(), "1");
    }
}
