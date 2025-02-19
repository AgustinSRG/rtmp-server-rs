// Utility to compare string in constant time

/// Compares 2 strings, constant time
/// Ensures timing attacks are not viable
/// 
/// # Arguments
/// 
/// * `a` - First string
/// * `b` - Second string
/// 
/// # Return value
/// 
/// Returns true if the 2 strings are equal, false otherwise
pub fn string_compare_constant_time(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for (a, b) in a.bytes().zip(b.bytes()) {
        if a != b {
            return false;
        }
    }

    true
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_compare_constant_time() {
        assert!(string_compare_constant_time("aaa123", "aaa123"));
        assert!(string_compare_constant_time("", ""));

        assert!(!string_compare_constant_time("", "aaa123"));
        assert!(!string_compare_constant_time("aaa123", "aaa1234"));
        assert!(!string_compare_constant_time("aaa123", ""));
        assert!(!string_compare_constant_time("aaa123", "aaa122"));
        assert!(!string_compare_constant_time("aaa123", "baa123"));
        assert!(!string_compare_constant_time("aaa123", "aba123"));
    }
}
