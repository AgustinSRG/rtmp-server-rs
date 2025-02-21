// Server status

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

mod channel_status;
mod set_publisher;
mod remove_publisher;
mod kill_publisher;
mod remove_all_publishers;
mod try_clear_channel;
mod add_player;
mod remove_player;
mod set_channel_metadata;
mod player_set_receive;
mod player_pause;
mod player_resume;

pub use channel_status::*;
pub use set_publisher::*;
pub use remove_publisher::*;
pub use kill_publisher::*;
pub use remove_all_publishers::*;
pub use try_clear_channel::*;
pub use add_player::*;
pub use remove_player::*;
pub use set_channel_metadata::*;
pub use player_set_receive::*;
pub use player_pause::*;
pub use player_resume::*;

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
