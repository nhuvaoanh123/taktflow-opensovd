# Edge-Case And Interop Suite

This suite is the Phase 11 repo-side gate for the standard-vs-extra boundary,
header compatibility, and fail-closed behavior around unsupported standard
families.

Run the suite from `opensovd-core/`:

```bash
cargo test --locked -p integration-tests --test phase9_auth_profiles
cargo test --locked -p integration-tests --test phase11_conformance_interop
```

The canonical suite descriptor is `suite.yaml` in this directory.
