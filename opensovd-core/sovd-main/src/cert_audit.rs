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

#![allow(clippy::doc_markdown)]

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

#[derive(Debug, Clone)]
pub struct CertAuditConfig {
    pub sqlite_path: String,
    pub file_path: String,
    pub dlt_context: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CertAuditEvent {
    pub timestamp: String,
    pub kind: String,
    pub serial: String,
    pub common_name: String,
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CertAuditEvent {
    #[must_use]
    pub fn new(
        kind: impl Into<String>,
        serial: impl Into<String>,
        common_name: impl Into<String>,
        profile: impl Into<String>,
        not_after: Option<String>,
        reason: Option<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            kind: kind.into(),
            serial: serial.into(),
            common_name: common_name.into(),
            profile: profile.into(),
            not_after,
            reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertAuditOutcome {
    pub file_written: bool,
    pub sqlite_written: bool,
    pub tracing_emitted: bool,
}

pub async fn record_cert_audit_event(
    config: &CertAuditConfig,
    event: &CertAuditEvent,
) -> Result<CertAuditOutcome, Box<dyn std::error::Error>> {
    let file_written = append_ndjson_line(&config.file_path, event)?;
    let sqlite_written = insert_sqlite_event(&config.sqlite_path, event).await?;

    tracing::info!(
        dlt_context = %config.dlt_context,
        cert_event_kind = %event.kind,
        cert_serial = %event.serial,
        cert_common_name = %event.common_name,
        cert_profile = %event.profile,
        cert_not_after = ?event.not_after,
        cert_reason = ?event.reason,
        "certificate lifecycle event"
    );

    Ok(CertAuditOutcome {
        file_written,
        sqlite_written,
        tracing_emitted: true,
    })
}

fn append_ndjson_line(
    path: &str,
    event: &CertAuditEvent,
) -> Result<bool, Box<dyn std::error::Error>> {
    let target = Path::new(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(target)?;
    let mut line = serde_json::to_string(event)?;
    line.push('\n');
    file.write_all(line.as_bytes())?;
    Ok(true)
}

async fn insert_sqlite_event(
    path: &str,
    event: &CertAuditEvent,
) -> Result<bool, Box<dyn std::error::Error>> {
    let target = Path::new(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let options = SqliteConnectOptions::new()
        .filename(target)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    ensure_sqlite_schema(&pool).await?;
    sqlx::query(
        "INSERT INTO cert_audit_events \
         (timestamp, kind, serial, common_name, profile, not_after, reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&event.timestamp)
    .bind(&event.kind)
    .bind(&event.serial)
    .bind(&event.common_name)
    .bind(&event.profile)
    .bind(&event.not_after)
    .bind(&event.reason)
    .execute(&pool)
    .await?;
    Ok(true)
}

async fn ensure_sqlite_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cert_audit_events (
            row_id       INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp    TEXT NOT NULL,
            kind         TEXT NOT NULL,
            serial       TEXT NOT NULL,
            common_name  TEXT NOT NULL,
            profile      TEXT NOT NULL,
            not_after    TEXT,
            reason       TEXT
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn record_cert_audit_event_writes_file_and_sqlite() {
        let dir = tempdir().expect("tempdir");
        let config = CertAuditConfig {
            sqlite_path: dir.path().join("cert-audit.db").display().to_string(),
            file_path: dir.path().join("audit.ndjson").display().to_string(),
            dlt_context: "AUDT".to_owned(),
        };
        let event = CertAuditEvent::new(
            "issue",
            "01AB",
            "observer-01",
            "mtls-client",
            Some("2027-04-22T00:00:00Z".to_owned()),
            None,
        );

        let outcome = record_cert_audit_event(&config, &event)
            .await
            .expect("record event");

        assert!(outcome.file_written);
        assert!(outcome.sqlite_written);
        assert!(outcome.tracing_emitted);

        let ndjson = std::fs::read_to_string(&config.file_path).expect("read ndjson");
        assert!(ndjson.contains("\"kind\":\"issue\""));

        let options = SqliteConnectOptions::new()
            .filename(&config.sqlite_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("sqlite connect");
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cert_audit_events")
            .fetch_one(&pool)
            .await
            .expect("row count");
        assert_eq!(row_count, 1);
    }
}
