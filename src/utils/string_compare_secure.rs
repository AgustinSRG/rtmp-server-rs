// Utility to compare string in constant time

use sha2::{Digest, Sha256};

/// Compares 2 strings by hashing them.
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
pub fn string_compare_time_safe(a: &str, b: &str) -> bool {
    let a_hash = Sha256::digest(a);
    let b_hash = Sha256::digest(b);

    for (a, b) in a_hash.into_iter().zip(b_hash) {
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
        assert!(string_compare_time_safe("aaa123", "aaa123"));
        assert!(string_compare_time_safe("", ""));

        assert!(!string_compare_time_safe("", "aaa123"));
        assert!(!string_compare_time_safe("aaa123", "aaa1234"));
        assert!(!string_compare_time_safe("aaa123", ""));
        assert!(!string_compare_time_safe("aaa123", "aaa122"));
        assert!(!string_compare_time_safe("aaa123", "baa123"));
        assert!(!string_compare_time_safe("aaa123", "aba123"));
    }
}
