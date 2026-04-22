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

use std::{env, sync::Arc};

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let bind_addr =
        env::var("TAKTFLOW_PHASE8_ML_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:21092".to_owned());
    let listener = TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|error| panic!("bind {bind_addr}: {error}"));
    let server = Arc::new(sovd_server::InMemoryServer::new_with_demo_data());
    let app = sovd_server::routes::app_with_server(server);
    println!("phase8_ml_sil_server listening on http://{bind_addr}");
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|error| panic!("serve {bind_addr}: {error}"));
}
