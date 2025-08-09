// Server status

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

mod add_player;
mod channel_status;
mod kill_publisher;
mod player_pause;
mod player_resume;
mod player_set_receive;
mod remove_all_publishers;
mod remove_player;
mod remove_publisher;
mod set_channel_metadata;
mod set_publisher;
mod try_clear_channel;

pub use add_player::*;
pub use channel_status::*;
pub use kill_publisher::*;
pub use player_pause::*;
pub use player_resume::*;
pub use player_set_receive::*;
pub use remove_all_publishers::*;
pub use remove_player::*;
pub use remove_publisher::*;
pub use set_channel_metadata::*;
pub use set_publisher::*;
pub use try_clear_channel::*;

/// Server status
pub struct RtmpServerStatus {
    /// Channels
    pub channels: HashMap<String, Arc<Mutex<RtmpChannelStatus>>>,
}

impl RtmpServerStatus {
    /// Creates new instance of RtmpServerStatus
    pub fn new() -> RtmpServerStatus {
        RtmpServerStatus {
            channels: HashMap::new(),
        }
    }
}
