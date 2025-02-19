// Environment variable utils

use std::env;

/// Gets boolean env var
///
/// # Arguments
/// 
/// * `var_name` - The name of the environment variable
/// * `default_val` - The default value
/// 
/// # Return value
///
/// Returns true if the value of the variable is 'YES' or 'TRUE' (Case insensitive)
/// Returns false if the value of the variable is 'NO' or 'FALSE' (Case insensitive)
/// Otherwise, returns the value of default_val
pub fn get_env_bool(var_name: &str, default_val: bool) -> bool {
    let r_var = env::var(var_name);

    match r_var {
        Ok(var_str) => {
            let var_str_upper = var_str.to_uppercase();

            match var_str_upper.as_str() {
                "YES" | "TRUE" => true,
                "NO" | "FALSE" => false,
                _ => default_val,
            }
        }
        Err(_) => default_val,
    }
}

/// Gets string env var
///
/// # Arguments
/// 
/// * `var_name` - The name of the environment variable
/// * `default_val` - The default value
/// 
/// # Return value
///
/// Returns The value of the variable as string or,
/// if not present, the value of default_val
pub fn get_env_string(var_name: &str, default_val: &str) -> String {
    let r_var = env::var(var_name);

    match r_var {
        Ok(r_var_str) => r_var_str,
        Err(_) => default_val.to_string(),
    }
}

/// Gets u32 env var
///
/// # Arguments
/// 
/// * `var_name` - The name of the environment variable
/// * `default_val` - The default value
/// 
/// # Return value
///
/// Returns The value of the variable as u32 or,
/// if not present or invalid, the value of default_val
pub fn get_env_u32(var_name: &str, default_val: u32) -> u32 {
    let r_var = env::var(var_name);

    match r_var {
        Ok(var_str) => {
            let r_num: Result<u32, _> = var_str.parse();

            match r_num {
                Ok(num) => num,
                Err(_) => default_val,
            }
        }
        Err(_) => default_val,
    }
}
