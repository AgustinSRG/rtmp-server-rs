// Redis command


/// RTMP command received via REdis
pub enum RedisRtmpCommand {
    KillSession{
        channel:String,
    },
    CloseStream {
        channel:String,
        stream_id: String,
    },
    Unknown,
}

impl RedisRtmpCommand {
    /// Parses command from string message
    pub fn parse(s: &str) -> RedisRtmpCommand {
        let parts: Vec<&str> = s.split(">").collect();

        if parts.len() < 2 {
            return RedisRtmpCommand::Unknown;
        }

        let cmd = parts[0].to_lowercase();
        let args_str = parts[1..].join(">");
        let args: Vec<&str> = args_str.split("|").collect();

        match cmd.as_str() {
            "kill-session" => {
                if args.len() < 1 {
                    return RedisRtmpCommand::Unknown;
                }

                RedisRtmpCommand::KillSession { channel: args[0].to_string() }
            }
            "close-stream" => {
                if args.len() < 2 {
                    return RedisRtmpCommand::Unknown;
                }

                RedisRtmpCommand::CloseStream { channel: args[0].to_string(), stream_id: args[1].to_string() }
            }
            _ => RedisRtmpCommand::Unknown
        }
    }
}
