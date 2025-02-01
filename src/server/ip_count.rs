// IP address connection counter

use std::{collections::HashMap, net::IpAddr};

use super::RtmpServerConfiguration;

/// IP connection counter
pub struct IpConnectionCounter {
    /// Limit per IP address
    limit: usize,

    /// Counters map
    counters: HashMap<IpAddr, usize>,
}

impl IpConnectionCounter {
    /// Creates new IpConnectionCounter
    pub fn new(config: &RtmpServerConfiguration) -> IpConnectionCounter {
        IpConnectionCounter {
            limit: config.max_concurrent_connections_per_ip as usize,
            counters: HashMap::new(),
        }
    }

    /// Adds IP address, trying to fit it into the limit
    /// Returns true if accepted, false if rejected
    pub fn add(&mut self, ip: &IpAddr) -> bool {
        match self.counters.get(ip) {
            Some(old_count) => {
                if *old_count < self.limit {
                    let (new_counter, overflow) = (*old_count).overflowing_add(1);

                    if !overflow {
                        self.counters.insert(*ip, new_counter);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            None => {
                self.counters.insert(*ip, 1);
                true
            }
        }
    }

    /// Removes IP address
    pub fn remove(&mut self, ip: &IpAddr) {
        if let Some(old_count) = self.counters.get(ip) {
            if *old_count > 0 {
                self.counters.insert(*ip, *old_count - 1);
            } else {
                self.counters.remove(ip);
            }
        }
    }
}
