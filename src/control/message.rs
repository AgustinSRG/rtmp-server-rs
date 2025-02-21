// Control server messages

use std::collections::HashMap;

/// Control server message
pub struct ControlServerMessage {
    /// Message type
    pub msg_type: String,

    /// Parameters
    pub parameters: Option<HashMap<String, String>>,
}

impl ControlServerMessage {
    /// Creates new ControlServerMessage with no parameters
    pub fn new(msg_type: String) -> ControlServerMessage {
        ControlServerMessage {
            msg_type,
            parameters: None,
        }
    }

    /// Creates new ControlServerMessage with parameters
    pub fn new_with_parameters(
        msg_type: String,
        parameters: HashMap<String, String>,
    ) -> ControlServerMessage {
        ControlServerMessage {
            msg_type,
            parameters: Some(parameters),
        }
    }

    /// Parses a message from string
    pub fn parse(input: &str) -> ControlServerMessage {
        let input_header = input.split("\n\n").nth(0).unwrap_or(input);
        let lines: Vec<&str> = input_header.split("\n").filter(|l| !l.is_empty()).collect();

        if lines.is_empty() {
            return ControlServerMessage::new("".to_string());
        }

        let msg_type = lines[0].to_uppercase();

        if lines.len() == 1 {
            return ControlServerMessage::new(msg_type);
        }

        let mut parameters: HashMap<String, String> = HashMap::new();

        for line in &lines[1..] {
            let line_parts: Vec<&str> = line.split(":").collect();

            if line_parts.len() < 2 {
                continue;
            }

            parameters.insert(line_parts[0].to_lowercase(), line_parts[1..].join(":"));
        }

        ControlServerMessage::new_with_parameters(msg_type, parameters)
    }

    /// Serializes message top string,
    /// in order to send it to the control server
    pub fn serialize(&self) -> String {
        let mut res = self.msg_type.to_uppercase();

        if let Some(parameters) = &self.parameters {
            for (key, val) in parameters {
                res.push_str(&format!("\n{}: {}", key, val));
            }
        }

        res
    }

    /// Gets the value of a parameter of the message
    pub fn get_parameter(&self, param_name: &str) -> Option<&str> {
        if let Some(parameters) = &self.parameters {
            parameters
                .get(&param_name.to_lowercase())
                .map(|x| x.as_str())
        } else {
            None
        }
    }
}
