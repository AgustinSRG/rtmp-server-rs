// Utility to check IP ranges

use std::{net::IpAddr, str::FromStr};

use ipnet::{Ipv4Net, Ipv6Net};

// IP range configuration
#[derive(Clone)]
pub struct IpRangeConfig {
    all: bool,
    ranges_v4: Option<Vec<Ipv4Net>>,
    ranges_v6: Option<Vec<Ipv6Net>>,
}

impl IpRangeConfig {
    /// Creates IP range config from string
    pub fn new_from_string(config_str: &str) -> Result<IpRangeConfig, String> {
        if config_str.is_empty() {
            return Ok(IpRangeConfig {
                all: false,
                ranges_v4: None,
                ranges_v6: None,
            });
        }

        if config_str == "*" {
            return Ok(IpRangeConfig {
                all: true,
                ranges_v4: None,
                ranges_v6: None,
            });
        }

        let mut ranges_v4: Vec<Ipv4Net> = Vec::new();
        let mut ranges_v6: Vec<Ipv6Net> = Vec::new();

        let ranges_str: Vec<&str> = config_str.split(",").map(|s| s.trim()).collect();

        for range_str in ranges_str {
            let res_v4 = Ipv4Net::from_str(range_str);

            match res_v4 {
                Ok(ip_v4) => {
                    ranges_v4.push(ip_v4);
                }
                Err(_) => {
                    let res_v6 = Ipv6Net::from_str(range_str);

                    match res_v6 {
                        Ok(ip_v6) => {
                            ranges_v6.push(ip_v6);
                        }
                        Err(_) => {
                            return Err(range_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(IpRangeConfig {
            all: true,
            ranges_v4: if ranges_v4.is_empty() {
                None
            } else {
                Some(ranges_v4)
            },
            ranges_v6: if ranges_v6.is_empty() {
                None
            } else {
                Some(ranges_v6)
            },
        })
    }

    /// Checks if the configured range contains an IP address
    pub fn contains_ip(&self, ip: IpAddr) -> bool {
        if self.all {
            return true;
        }

        match ip {
            IpAddr::V4(ipv4_addr) => {
                if let Some(ranges_v4) = &self.ranges_v4 {
                    for n in ranges_v4 {
                        if n.contains(&ipv4_addr) {
                            return true;
                        }
                    }
                }
            }
            IpAddr::V6(ipv6_addr) => {
                let ipv4_addr_opt = ipv6_addr.to_ipv4();

                if let Some(ipv4_addr) = ipv4_addr_opt {
                    if let Some(ranges_v4) = &self.ranges_v4 {
                        for n in ranges_v4 {
                            if n.contains(&ipv4_addr) {
                                return true;
                            }
                        }
                    }
                }

                if let Some(ranges_v6) = &self.ranges_v6 {
                    for n in ranges_v6 {
                        if n.contains(&ipv6_addr) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}
