// ID validation

/// Default ID length limit
pub const DEFAULT_MAX_ID_LENGTH: usize = 128;

/// Validates ID as string
/// 
/// # Arguments
/// 
/// * `id` - ID to validate
/// * `max_len` - Max allowed length for identifiers
/// 
/// # Return value
/// 
/// Returns true if the ID is valid, false otherwise
pub fn validate_id_string(id: &str, max_len: usize) -> bool {
    if id.is_empty() || id.len() > max_len {
        return false;
    }

    for char in id.chars() {
        match char {
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {}
            'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'I' | 'J' | 'K' | 'L' | 'M' | 'N'
            | 'O' | 'P' | 'Q' | 'R' | 'S' | 'T' | 'U' | 'V' | 'W' | 'X' | 'Y' | 'Z' => {}
            'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'h' | 'i' | 'j' | 'k' | 'l' | 'm' | 'n'
            | 'o' | 'p' | 'q' | 'r' | 's' | 't' | 'u' | 'v' | 'w' | 'x' | 'y' | 'z' => {}
            '-' | '_' => {}
            _ => return false,
        }
    }

    true
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_id_string() {
        let max_len = 32;

        assert!(!validate_id_string("", max_len));
        assert!(!validate_id_string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", max_len));
        assert!(validate_id_string("a", max_len));
        assert!(validate_id_string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", max_len));
        assert!(validate_id_string("abc-DEF-1234567890_", max_len));
    }
}
