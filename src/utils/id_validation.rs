// ID validation

use crate::utils::{get_env_bool, get_env_u32};

/// Default ID length limit
pub const DEFAULT_MAX_ID_LENGTH: usize = 128;

/// ID validation configuration
#[derive(Clone)]
pub struct IdValidationConfig {
    /// Max length for IDs
    max_len: usize,

    /// True to allow empty strings as IDs
    allow_empty_string: bool,

    /// TRue to allow special characters in IDs
    allow_special_characters: bool,
}

impl IdValidationConfig {
    /// Loads configuration for environment variables
    pub fn load_from_env() -> IdValidationConfig {
        let max_len = get_env_u32("ID_MAX_LENGTH", DEFAULT_MAX_ID_LENGTH as u32) as usize;

        let allow_empty_string = get_env_bool("ID_ALLOW_EMPTY", false);
        let allow_special_characters = get_env_bool("ID_ALLOW_SPECIAL_CHARACTERS", false);

        IdValidationConfig {
            max_len,
            allow_empty_string,
            allow_special_characters,
        }
    }
}

/// Validates ID as string
///
/// # Arguments
///
/// * `id` - ID to validate
/// * `config` - ID validation configuration
///
/// # Return value
///
/// Returns true if the ID is valid, false otherwise
pub fn validate_id_string(id: &str, config: &IdValidationConfig) -> bool {
    if id.is_empty() && !config.allow_empty_string {
        return false;
    }

    if id.len() > config.max_len {
        return false;
    }

    if config.allow_special_characters {
        for char in id.chars() {
            match char {
                '>' | '|' | '\n' => {
                    return false;
                }
                _ => {}
            }
        }

        return true;
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
        // Defaults

        let mut config = IdValidationConfig {
            max_len: 32,
            allow_empty_string: false,
            allow_special_characters: false,
        };

        assert!(!validate_id_string("", &config));
        assert!(!validate_id_string(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &config
        ));
        assert!(!validate_id_string("a%", &config));

        assert!(validate_id_string("a", &config));
        assert!(validate_id_string(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &config
        ));
        assert!(validate_id_string("abc-DEF-1234567890_", &config));

        // Allow empty

        config.allow_empty_string = true;

        assert!(validate_id_string("", &config));
        assert!(!validate_id_string("a%", &config));

        assert!(validate_id_string("a", &config));
        assert!(validate_id_string(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &config
        ));
        assert!(validate_id_string("abc-DEF-1234567890_", &config));

        assert!(!validate_id_string(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &config
        ));

        // Allow special characters

        config.allow_special_characters = true;

        assert!(validate_id_string("", &config));
        assert!(validate_id_string("a%", &config));

        assert!(validate_id_string("a", &config));
        assert!(validate_id_string(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &config
        ));
        assert!(validate_id_string("abc-DEF-1234567890_", &config));

        assert!(!validate_id_string(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            &config
        ));
    }
}
