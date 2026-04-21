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
        }
    }
}
