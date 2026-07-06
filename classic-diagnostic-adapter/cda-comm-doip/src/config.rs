/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 */

use cda_interfaces::config::{ConfigSanity, ConfigSanityError};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DoipConfig {
    pub protocol_version: u8,
    pub tester_address: String,
    pub tester_subnet: String,
    pub static_gateway_ip: Option<String>,
    pub gateway_port: u16,
    pub tls_port: u16,
    pub send_timeout_ms: u64,
    pub enable_alive_check: bool,
    pub send_diagnostic_message_ack: bool,
    /// Interval in seconds between `DoIP` alive check requests sent on idle connections.
    /// The alive check is only sent when no diagnostic communication has occurred
    /// for this duration. Set to 0 to disable the alive check.
    pub alive_check_interval_secs: u64,
}

impl Default for DoipConfig {
    fn default() -> Self {
        Self {
            protocol_version: 0x02,
            tester_address: "127.0.0.1".to_owned(),
            tester_subnet: "255.255.0.0".to_owned(),
            static_gateway_ip: None,
            gateway_port: 13400,
            tls_port: 3496,
            send_timeout_ms: 1000,
            enable_alive_check: true,
            send_diagnostic_message_ack: true,
            alive_check_interval_secs: 1800, // 30 minutes
        }
    }
}

impl ConfigSanity for DoipConfig {
    fn validate_sanity(&self) -> Result<(), ConfigSanityError> {
        fn validate_ip(ip: &str, field: &str) -> Result<(), ConfigSanityError> {
            ip.parse::<std::net::IpAddr>().map(|_| ()).map_err(|_| {
                ConfigSanityError::InvalidValue {
                    field: field.to_owned(),
                    reason: format!("{ip} is neither a valid IPv4 nor IPv6 address"),
                }
            })
        }

        fn validate_port(port: u16, field: &str) -> Result<(), ConfigSanityError> {
            if port == 0 {
                return Err(ConfigSanityError::InvalidValue {
                    field: field.to_owned(),
                    reason: "Port must be greater than 0".to_string(),
                });
            }
            Ok(())
        }

        fn validate_timeout(timeout: u64, field: &str) -> Result<(), ConfigSanityError> {
            if timeout == 0 {
                return Err(ConfigSanityError::InvalidValue {
                    field: field.to_owned(),
                    reason: "Timeout must be greater than 0".to_string(),
                });
            }
            Ok(())
        }

        validate_ip(&self.tester_address, "tester_address")?;
        validate_ip(&self.tester_subnet, "tester_address")?;
        validate_port(self.gateway_port, "gateway_port")?;
        validate_port(self.tls_port, "tls_port")?;
        validate_timeout(self.send_timeout_ms, "send_timeout_ms")?;
        if self.alive_check_interval_secs > u64::from(u32::MAX) {
            return Err(ConfigSanityError::InvalidValue {
                field: "alive_check_interval_secs".to_owned(),
                reason: "Interval is too large, use 0 to disable it".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use doip_definitions::header::ProtocolVersion;

    #[test]
    fn protocol_version_from_u8() {
        let v2: u8 = 0x02;
        let v3: u8 = 0x03;

        assert_eq!(
            ProtocolVersion::try_from(&v2).unwrap(),
            ProtocolVersion::Iso13400_2012
        );
        assert_eq!(
            ProtocolVersion::try_from(&v3).unwrap(),
            ProtocolVersion::Iso13400_2019
        );
    }
}
