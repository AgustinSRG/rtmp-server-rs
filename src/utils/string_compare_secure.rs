// Utility to compare string in constant time

/// Compares 2 strings, constant time
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
