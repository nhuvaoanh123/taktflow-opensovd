# ISO 17978 Conformance Suite

This suite is the Phase 11 repo-side gate for the local ISO 17978 subset
declared in `docs/adr/ADR-0039-iso-17978-conformance-subset.md`.

It does not claim an official ISO conformance class. It verifies the
route-method subset, wire contracts, and selected edge behavior that the repo
currently declares under `/sovd/v1/components/...`.

Run the suite from `opensovd-core/`:

```bash
cargo test --locked -p integration-tests --test in_memory_mvp_flow
cargo test --locked -p integration-tests --test openapi_roundtrip
cargo test --locked -p integration-tests --test phase5_faults_pagination_contract
cargo test --locked -p integration-tests --test phase11_conformance_iso_17978
```

The canonical suite descriptor is `suite.yaml` in this directory.
