# COVESA VSS Spec-Drift Review 1

Date: 2026-04-23
Status: Accepted for internal use
Owner: Taktflow SOVD workstream

## Scope

This review covers the first checked-in semantic mapping slice in
`opensovd-core/sovd-covesa/schemas/vss-map.yaml`:

1. `Vehicle.OBD.DTCList`
2. `Vehicle.OBD.DTC.P0A1F`
3. `Vehicle.Powertrain.Battery.StateOfCharge`
4. `Vehicle.Powertrain.Battery.StateOfHealth`
5. `Vehicle.VersionVSS`
6. `Vehicle.Service.ClearDTCs`
7. `Vehicle.Service.Routine.motor_self_test.Start`

The goal is not to re-open ADR-0026. The goal is to record where the
checked-in first slice matches the current official VSS catalog and where it
is knowingly an internal compatibility overlay.

## Current repo pin

The checked-in mapping file declares:

- `schema_version: 1`
- `vss_version: v5.0`

Source: `opensovd-core/sovd-covesa/schemas/vss-map.yaml`

## External reference point used for this review

As checked on 2026-04-23:

1. the public COVESA documentation front page advertises **Latest Released
   Version: 6.0**
2. the current `master` `Vehicle.vspec` reports `VersionVSS.Major = 7`,
   `Minor = 0`, `Patch = 0`, `Label = dev`
3. the VSS 5.0 release notes say the `Vehicle.OBD` branch is deprecated in
   5.0 and planned for removal in 6.0

This means the repo's `v5.0` pin is intentionally behind both the latest
release and `master`.

## Findings

### 1. `Vehicle.VersionVSS` is still aligned

This row is the safest of the seven.

The current `master` `Vehicle.vspec` still defines the `VersionVSS` branch,
so the checked-in constant row remains semantically aligned even though the
repo pin is older than the latest public release.

Decision:

- keep this row as a stable standard-aligned mapping

### 2. `Vehicle.OBD.*` rows are a known standards-drift carryover

The 5.0 release notes explicitly deprecate the `Vehicle.OBD` branch and say
the plan is to remove it in 6.0. The same release notes recommend either:

1. moving to a suitable replacement signal elsewhere in the tree, or
2. keeping the signal in an overlay from VSS 6.0 onward

That means these two rows:

- `Vehicle.OBD.DTCList`
- `Vehicle.OBD.DTC.P0A1F`

should not be presented as "current standard VSS" rows. They are acceptable
as an internal compatibility bridge because Taktflow is translating an
existing SOVD diagnostic surface, but they are already on a deprecated branch
in the catalog version we pinned.

Decision:

- keep both rows for internal translation only
- treat both as overlay-compatible legacy rows, not as forward-looking public
  standard mappings

### 3. The battery rows are the highest-risk naming drift in the first slice

The current repo rows are:

- `Vehicle.Powertrain.Battery.StateOfCharge`
- `Vehicle.Powertrain.Battery.StateOfHealth`

I did not find a current official catalog source in the public VSS docs that
cleanly confirms those exact paths as the present standard names on
2026-04-23.

What the official sources do show is:

1. the low-voltage battery file on `master` lives at `spec/Vehicle/Battery.vspec`
2. the public release history includes battery-file renaming work

That is enough to say the battery namespace is moving and our first-slice
paths should be treated as a pinned internal compatibility choice, not as a
"safe to externalize unchanged" standards claim.

Decision:

- keep the two battery rows for the current internal demo slice
- require an explicit repin to one named official catalog release before
  exposing them as externally claimed standard VSS paths

### 4. The `Vehicle.Service.*` rows are overlays by design

The official VSS overview says vendor-specific extensions and adaptations are
allowed, and the official "Extending and Customizing VSS" guidance says the
overlay mechanism is the preferred way to apply changes on top of the
standard catalog.

I did not find an official standard-catalog source for:

- `Vehicle.Service.ClearDTCs`
- `Vehicle.Service.Routine.motor_self_test.Start`

That is acceptable. These rows are actuator-style control overlays owned by
the Taktflow mapping layer, not claims about the standard catalog.

Decision:

- keep both `Vehicle.Service.*` rows
- classify them as OEM / integration overlays, not standard-catalog rows

## Verdict

The current seven-row mapping is acceptable for **internal Taktflow semantic
translation** and test coverage, but it is not a clean "current COVESA VSS
standard subset" claim.

Classification by row:

| Row | Status |
|---|---|
| `Vehicle.VersionVSS` | safe standard-aligned row |
| `Vehicle.OBD.*` | deprecated-branch compatibility rows |
| `Vehicle.Powertrain.Battery.*` | internal pinned rows; repin required before external claim |
| `Vehicle.Service.*` | intentional overlay rows |

## Required posture after this review

1. Keep `opensovd-core/sovd-covesa/schemas/vss-map.yaml` as an internal
   contract file, not a standards marketing artifact.
2. Do not describe the current seven-row set as "the current COVESA standard
   subset" in external material.
3. When the mapping grows beyond the first slice, repin against one named VSS
   release and move deprecated or overlay-only rows into an explicitly named
   overlay section.

## Sources

- Repo pin: `opensovd-core/sovd-covesa/schemas/vss-map.yaml`
- COVESA VSS docs front page: https://covesa.github.io/vehicle_signal_specification/
- COVESA VSS extension guidance: https://covesa.github.io/vehicle_signal_specification/extensions/
- COVESA VSS overview: https://covesa.github.io/vehicle_signal_specification/introduction/overview/
- COVESA VSS `master` `Vehicle.vspec`:
  https://raw.githubusercontent.com/COVESA/vehicle_signal_specification/master/spec/Vehicle/Vehicle.vspec
- COVESA VSS releases page:
  https://github.com/COVESA/vehicle_signal_specification/releases
