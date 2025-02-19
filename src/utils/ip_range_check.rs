// Utility to check IP ranges

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use ipnet::{Ipv4Net, Ipv6Net};

// IP range configuration
// Represents a list of IP ranges
#[derive(Clone)]
pub struct IpRangeConfig {
    all: bool,

    ips_v4: Option<Vec<Ipv4Addr>>,
    ranges_v4: Option<Vec<Ipv4Net>>,

    ips_v6: Option<Vec<Ipv6Addr>>,
    ranges_v6: Option<Vec<Ipv6Net>>,
}

impl IpRangeConfig {
    /// Creates IP range config from string
    /// 
    /// # Arguments
    /// 
    /// * `config_str` - String configuration from environment
    /// 
    /// # Return value
    /// 
    /// A result for the config. In case of error, a sub-string
    /// of the invalid range is provided to indicate the user
    pub fn new_from_string(config_str: &str) -> Result<IpRangeConfig, String> {
        if config_str.is_empty() {
            return Ok(IpRangeConfig {
                all: false,
                ips_v4: None,
                ranges_v4: None,
                ips_v6: None,
                ranges_v6: None,
            });
        }

        if config_str == "*" {
            return Ok(IpRangeConfig {
                all: true,
                ips_v4: None,
                ranges_v4: None,
                ips_v6: None,
                ranges_v6: None,
            });
        }

        let mut ips_v4: Vec<Ipv4Addr> = Vec::new();
        let mut ips_v6: Vec<Ipv6Addr> = Vec::new();

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
                    let res_ip_v4 = Ipv4Addr::from_str(range_str);

                    match res_ip_v4 {
                        Ok(ip_v4) => {
                            ips_v4.push(ip_v4);
                        }
                        Err(_) => {
                            let res_v6 = Ipv6Net::from_str(range_str);

                            match res_v6 {
                                Ok(ip_v6) => {
                                    ranges_v6.push(ip_v6);
                                }
                                Err(_) => {
                                    let res_ip_v6 = Ipv6Addr::from_str(range_str);

                                    match res_ip_v6 {
                                        Ok(ip_v6) => {
                                            ips_v6.push(ip_v6);
                                        }
                                        Err(_) => {
                                            return Err(range_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(IpRangeConfig {
            all: false,
            ips_v4: if ips_v4.is_empty() {
                None
            } else {
                Some(ips_v4)
            },
            ranges_v4: if ranges_v4.is_empty() {
                None
            } else {
                Some(ranges_v4)
            },
            ips_v6: if ips_v6.is_empty() {
                None
            } else {
                Some(ips_v6)
            },
            ranges_v6: if ranges_v6.is_empty() {
                None
            } else {
                Some(ranges_v6)
            },
        })
    }

    /// Checks if IP (V4) is included in the range
    fn check_ip_v4(&self, ipv4_addr: &Ipv4Addr) -> bool {
        if let Some(ips_v4) = &self.ips_v4 {
            for n in ips_v4 {
                if n == ipv4_addr {
                    return true;
                }
            }
        }
        if let Some(ranges_v4) = &self.ranges_v4 {
            for n in ranges_v4 {
                if n.contains(ipv4_addr) {
                    return true;
                }
            }
        }

        false
    }

    /// Checks if IP (V6) is included in the range
    fn check_ip_v6(&self, ipv6_addr: &Ipv6Addr) -> bool {
        if let Some(ips_v6) = &self.ips_v6 {
            for n in ips_v6 {
                if n == ipv6_addr {
                    return true;
                }
            }
        }

        if let Some(ranges_v6) = &self.ranges_v6 {
            for n in ranges_v6 {
                if n.contains(ipv6_addr) {
                    return true;
                }
            }
        }

        let ipv4_addr_opt = ipv6_addr.to_ipv4();

        if let Some(ipv4_addr) = ipv4_addr_opt {
            if self.check_ip_v4(&ipv4_addr) {
                return true;
            }
        }

        false
    }

    /// Checks if the configured range contains an IP address
    /// 
    /// # Arguments
    /// 
    /// * `ip` - The IP address to check
    /// 
    /// # Return value
    /// 
    /// Returns true if the IP is contained in the range, false otherwise
    pub fn contains_ip(&self, ip: &IpAddr) -> bool {
        if self.all {
            return true;
        }

        match ip {
            IpAddr::V4(ipv4_addr) => {
                if self.check_ip_v4(ipv4_addr) {
                    return true
                }
            }
            IpAddr::V6(ipv6_addr) => {
                if self.check_ip_v6(ipv6_addr) {
                    return true
                }
            }
        }

        false
    }
}

// Tests

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn test_ip_range_util() {
        let ip_v4_1 = IpAddr::V4(Ipv4Addr::from_str("127.0.0.1").unwrap());
        let ip_v4_2 = IpAddr::V4(Ipv4Addr::from_str("10.0.0.1").unwrap());

        let ip_v6_1 = IpAddr::V6(Ipv6Addr::from_str("::1").unwrap());
        let ip_v6_2 =
            IpAddr::V6(Ipv6Addr::from_str("2001:db8:abcd:0012:1319:8a2e:0370:7344").unwrap());

        let range_1 = IpRangeConfig::new_from_string("").unwrap();

        assert!(!range_1.contains_ip(&ip_v4_1));
        assert!(!range_1.contains_ip(&ip_v4_2));
        assert!(!range_1.contains_ip(&ip_v6_1));
        assert!(!range_1.contains_ip(&ip_v6_2));

        let range_2 = IpRangeConfig::new_from_string("*").unwrap();

        assert!(range_2.contains_ip(&ip_v4_1));
        assert!(range_2.contains_ip(&ip_v4_2));
        assert!(range_2.contains_ip(&ip_v6_1));
        assert!(range_2.contains_ip(&ip_v6_2));

        let range_3 = IpRangeConfig::new_from_string("10.0.0.0/8").unwrap();

        assert!(!range_3.contains_ip(&ip_v4_1));
        assert!(range_3.contains_ip(&ip_v4_2));
        assert!(!range_3.contains_ip(&ip_v6_1));
        assert!(!range_3.contains_ip(&ip_v6_2));

        let range_4 = IpRangeConfig::new_from_string("10.0.0.0/8,127.0.0.1,::1").unwrap();

        assert!(range_4.contains_ip(&ip_v4_1));
        assert!(range_4.contains_ip(&ip_v4_2));
        assert!(range_4.contains_ip(&ip_v6_1));
        assert!(!range_4.contains_ip(&ip_v6_2));

        let range_5 = IpRangeConfig::new_from_string("10.0.0.0/8,2001:db8:abcd:0012::/64").unwrap();

        assert!(!range_5.contains_ip(&ip_v4_1));
        assert!(range_5.contains_ip(&ip_v4_2));
        assert!(!range_5.contains_ip(&ip_v6_1));
        assert!(range_5.contains_ip(&ip_v6_2));
    }
}
