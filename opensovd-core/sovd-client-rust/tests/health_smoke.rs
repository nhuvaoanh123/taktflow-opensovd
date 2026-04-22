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

use sovd_client_rust::SovdClient;
use sovd_server::{InMemoryServer, routes};
use tokio::net::TcpListener;

#[tokio::test]
async fn sdk_health_smoke_round_trips_against_in_memory_server() {
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
    let health = client.health().await.expect("health response");

    assert_eq!(health.status, "ok");
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));

    server_task.abort();
}
