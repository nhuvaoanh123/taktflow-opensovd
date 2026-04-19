# Reference Model Layout

ADR-0028 pins the first Edge ML artifact layout for `sovd-ml`:

- `opensovd-core/sovd-ml/models/reference-fault-predictor.onnx`
- `opensovd-core/sovd-ml/models/reference-fault-predictor.sig`

This scaffold deliberately reserves those exact locations before any real
model artifact is checked in. `UP3-05` is the first slice that must
replace the reserved layout with a real signed model and prove
verify-before-load behavior in SIL.
