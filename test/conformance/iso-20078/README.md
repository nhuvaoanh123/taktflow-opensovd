# ISO 20078 Conformance Suite

This suite is the Phase 11 repo-side gate for the partial Extended Vehicle
surface declared in `docs/adr/ADR-0027-extended-vehicle-scope.md`.

It verifies the repo's ISO-20078-shaped REST and MQTT slice only. It does not
claim full ISO 20078 coverage.

Run the suite from `opensovd-core/`:

```bash
cargo test --locked -p integration-tests --test extended_vehicle_rest_surface
cargo test --locked -p integration-tests --test extended_vehicle_mqtt_publish
cargo test --locked -p integration-tests --test extended_vehicle_mqtt_subscribe
cargo test --locked -p integration-tests --test extended_vehicle_fault_log_sil_scenario
cargo test --locked -p integration-tests --test extended_vehicle_state_sil_scenario
cargo test --locked -p integration-tests --test phase11_conformance_iso_20078
```

The canonical suite descriptor is `suite.yaml` in this directory.
