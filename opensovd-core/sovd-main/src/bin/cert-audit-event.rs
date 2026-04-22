/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 */

use clap::Parser;
use sovd_main::cert_audit::{CertAuditConfig, CertAuditEvent, record_cert_audit_event};

#[derive(Parser, Debug)]
#[command(name = "cert-audit-event", about = "Write one certificate lifecycle audit event")]
struct Args {
    #[arg(long)]
    sqlite: String,
    #[arg(long)]
    file: String,
    #[arg(long)]
    kind: String,
    #[arg(long)]
    serial: String,
    #[arg(long = "common-name")]
    common_name: String,
    #[arg(long)]
    profile: String,
    #[arg(long = "not-after")]
    not_after: Option<String>,
    #[arg(long)]
    reason: Option<String>,
}

fn parse_bool_env(name: &str) -> bool {
    std::env::var(name).ok().is_some_and(|value| {
        matches!(
            value.as_str(),
            "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
        )
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let tracing_config = sovd_tracing::TracingConfig {
        filter_directive: "info".to_owned(),
        dlt: sovd_tracing::DltConfig {
            enabled: parse_bool_env("CERT_AUDIT_DLT_ENABLED"),
            app_id: std::env::var("CERT_AUDIT_DLT_APP_ID").unwrap_or_else(|_| "SOVD".to_owned()),
            app_description: std::env::var("CERT_AUDIT_DLT_APP_DESCRIPTION")
                .unwrap_or_else(|_| "OpenSOVD certificate audit".to_owned()),
        },
        otel: sovd_tracing::OtelConfig::default(),
    };
    let _guard = sovd_tracing::init(&tracing_config)?;

    let event = CertAuditEvent::new(
        args.kind,
        args.serial,
        args.common_name,
        args.profile,
        args.not_after,
        args.reason,
    );
    let config = CertAuditConfig {
        sqlite_path: args.sqlite,
        file_path: args.file,
        dlt_context: "AUDT".to_owned(),
    };
    record_cert_audit_event(&config, &event).await?;
    Ok(())
}
