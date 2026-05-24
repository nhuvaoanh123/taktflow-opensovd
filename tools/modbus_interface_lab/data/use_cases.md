# BMS Interface E2E Use Cases - Modbus Interface

Source images:

- `IMG_3140.HEIC`
- `IMG_3141.HEIC`
- `IMG_3142.HEIC`
- `IMG_3143.HEIC`

Note: the source photos show the `Priority` column header, but no priority values are visible. Row 7 is partially obscured at the top of the photo, so only visible text is captured there.

Call Tendency monitor BMS health and read/write all registers marked as accessible in the Modbus specification.

## BMS Interface Use Cases

| ID | Interface scenario | Available Test | Status | Priority |
| --- | --- | --- | --- | --- |
| 1 | **PC dashboard via Ethernet to BMS over Modbus TCP** - Continuous monitoring of BMS health KPIs and key SunSpec/Modbus registers; **Read/write** of all registers marked accessible in the Modbus spec; **Verification** of update rates, data validity, error handling/timeouts, and persistence where applicable. |  | To be reviewed for gap |  |
| 2 | **Contactor control via SunSpec M804** - **Write Close/open contactors command** via SetCon/SetEna; **Verify** contactor state feedback (ConSt bits); **Validate** precharge sequence timing and voltage thresholds; **Verify** ConFail detection and fault reaction; **Confirm** no contactor close allowed during SW flashing. |  | To be reviewed for gap |  |
| 3 | **Fault monitoring & alarm reset** - **Read** Evt1/EvtVnd1/2/CusEvt1-5 from M802, M804, M64093; **Verify** L2/L3 fault escalation and debounce timing; **Heal** faults and confirm auto-clear of L2 events; **Write** AlmRst to clear latched L3 events; **Verify** fault-free state restored. |  | To be reviewed for gap |  |
| 4 | **SOC/SOH/SOP/SOE monitoring** - **Read** state estimates via SunSpec; **Verify** DCIR values; **Validate** power prediction registers (charge/discharge limits); **Cross-check** energy throughput counters against expected values. |  | To be reviewed for gap |  |
| 5 | **Isolation monitoring control** - **Write Enable/disable command** via M64093 IsoMonEnable; **Read** IsoMonResMea and IsoMonStatus; **Verify** ground fault detection triggers correct event bits; **Confirm** isolation monitoring resumes correctly after disable/re-enable cycle. |  | To be reviewed for gap |  |
| 6 | **Power saving mode** - **Write Enter command** via M64093 EnterPowerSaving; **Verify** BMS reduces power consumption and stops non-essential tasks; **Exit and confirm** full operational capability resumes; validate wake-up triggers and timing. |  | To be reviewed for gap |  |
| 7 | **Partially obscured in source image** - visible text: aggregates in M804; **Cross-check** against CMB simulator setpoints. |  | Not visible in source image |  |
| 8 | **SW flashing via Ethernet** - **Firmware update** via DoIP; verify bank swap (A->B); confirm data retention across flash (NVM, calibration, counters); **Validate** parallel flash for multi-ECU setups; **Verify** no contactor close during flash. |  | To be reviewed for gap |  |
| 9 | **NVM & diagnostic data readback** - **Read** histograms (cell V, T, current distributions); **Read** freeze-frame data at fault occurrence; **Verify** energy throughput counters (charge/discharge Wh); **Validate** data persistence across power cycles; **Confirm** correct timestamp and event correlation. |  | To be reviewed for gap |  |
| 10 | **Device ID configuration** - **Write DA** via M1 register; **Verify** DA persists across power cycle; **Confirm** BMS responds on new address after reconfiguration; **Validate** address conflict detection if applicable. |  | To be reviewed for gap |  |
| 11 | **Controller heartbeat & connection management** - **Write CtrlHb** via M802 to maintain connection; **Verify** timeout behaviour when heartbeat stops; **Validate** up to 4 simultaneous TCP connections; **Confirm** graceful handling of connection drops and reconnects. |  | To be reviewed for gap |  |
| 12 | **Cell balancing status & thermal constraints** - **Read** per-cell balancing active/inactive status; **Verify** balancing disabled when cell T exceeds thermal safety threshold; **Confirm** balancing resumes in safe range; **Validate** balancing current and duration reporting. |  | To be reviewed for gap |  |
| 13 | **Service tool operations** - GUI/CLI for commissioning, calibration, debug; **Verify** read/write of calibration parameters; **Confirm** diagnostic data export; **Validate** service tool connectivity over Ethernet. |  | To be reviewed for gap |  |
