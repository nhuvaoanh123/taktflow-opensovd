<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0
-->

# Phase 6 OTLP Production Path

`P6-03` extends the earlier one-binary OTLP spike into the real forwarded
request path that ships today:

1. `sovd-main` creates the ingress HTTP request span.
2. `sovd-server::backends::CdaBackend` creates a child `cda.forward` span.
3. The forwarder injects W3C `traceparent` context into the outbound CDA HTTP
   request.
4. `classic-diagnostic-adapter/cda-sovd` extracts that context and attaches its
   own HTTP request span to the same trace.

The result is one OTLP trace that covers the request as it crosses the
`sovd-main -> CDA` boundary.

## Config

Enable OTLP export in both processes.

`sovd-main`:

```toml
[logging.otel]
enabled = true
endpoint = "http://127.0.0.1:4317"
service_name = "sovd-main"
```

`opensovd-cda`:

```toml
[logging.otel]
enabled = true
endpoint = "http://127.0.0.1:4317"
```

Notes:

- `sovd-main` can override `service_name`; the default is `sovd-main`.
- CDA currently reports its OTLP resource service name from the `cda-tracing`
  crate, so Jaeger will show it as `cda-tracing`.
- The request you verify must hit a forwarded component, not a local-only one.

## Verifier Flow

1. Start Jaeger OTLP gRPC on `127.0.0.1:4317`:

```powershell
docker run -d --name jaeger-prod-path -p 4317:4317 -p 4318:4318 -p 16686:16686 jaegertracing/all-in-one:latest
```

2. Start CDA with `[logging.otel]` enabled and a config that serves at least one
   component through `vehicle/v15`.
3. Start `sovd-main` with `[logging.otel]` enabled and a matching `[[cda_forward]]`
   entry pointing at that CDA instance.
4. Trigger a forwarded request, for example:

```powershell
curl.exe http://127.0.0.1:20002/sovd/v1/components/cvc/faults
```

5. Query Jaeger:

```powershell
curl.exe "http://127.0.0.1:16686/api/traces?service=sovd-main&limit=20"
curl.exe "http://127.0.0.1:16686/api/traces?service=cda-tracing&limit=20"
```

Expected result:

- one trace id is shared by the `sovd-main` request span, the `cda.forward`
  client span, and the downstream CDA `request` span
- the trace proves the production forward path is joined end-to-end instead of
  producing two unrelated roots

## Verification Status On This Windows Host

- `cargo check -p sovd-server -p sovd-main` passed
- `cargo test -p sovd-server -p sovd-main` passed
- `cargo check -p cda-sovd` passed
- `cargo test -p cda-sovd` passed
- `cargo check -p opensovd-cda` is still blocked on missing native OpenSSL for
  `openssl-sys` on this Windows MSVC host
