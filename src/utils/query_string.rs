// Query string utilities

use std::collections::HashMap;

/// Parses query string (does not parse parameters)
pub fn parse_query_string_simple(query_string: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    if query_string.len() > 0 {
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
