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

use std::sync::Arc;

use sovd_client_rust::{SdkError, SovdClient};
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

#[tokio::test]
async fn sdk_discovers_versions_and_selects_v1() {
    let server = Arc::new(InMemoryServer::new_with_demo_data());
    let app = routes::app_with_server(server);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback port");
    let addr = listener.local_addr().expect("listener addr");
    let server_task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve in-memory app");
    });

    let client = SovdClient::new(format!("http://{addr}")).expect("build sdk client");

    let info = client.version_info().await.expect("version-info response");
    assert_eq!(info.sovd_info.len(), 1);
    assert_eq!(info.sovd_info[0].version, "v1");
    assert_eq!(info.sovd_info[0].base_uri, "/sovd/v1");

    let v1 = client
        .select_version(|s| s.version == "v1")
        .await
        .expect("select v1 instance");
    assert!(v1.base_url().path().contains("/sovd/v1"));

    let missing = client.select_version(|s| s.version == "v99").await;
    assert!(matches!(missing, Err(SdkError::NoMatchingVersion)));

    server_task.abort();
}
