# Glossary

Domain-specific terms used in taktflow-opensovd documentation and source code.

## Automotive diagnostics

| Term | Definition |
|------|-----------|
| **ASAM** | Association for Standardization of Automation and Measuring Systems. Publishes SOVD (ISO 17978) and ODX standards. |
| **ASIL** | Automotive Safety Integrity Level (ISO 26262). Ranges from QM (no safety requirement) through ASIL-A to ASIL-D (most stringent). |
| **CAN** | Controller Area Network. Serial bus standard for ECU communication. Operates at up to 1 Mbit/s. |
| **CDA** | Classic Diagnostic Adapter. OpenSOVD component that translates SOVD REST calls into UDS/DoIP for legacy ECUs. |
| **DBC** | CAN database file format. Describes CAN message definitions and signal layouts. |
| **DFM** | Diagnostic Fault Manager. Manages fault ingestion, persistence, and query via the SOVD API. |
| **DID** | Data Identifier. UDS concept for reading/writing ECU data (e.g., HW version, calibration values). |
| **DLT** | Diagnostic Log and Trace. COVESA standard for structured automotive logging. |
| **DoIP** | Diagnostics over Internet Protocol (ISO 13400). Tunnels UDS messages over TCP/IP. |
| **DTC** | Diagnostic Trouble Code. Standardized fault codes stored by ECUs. In SOVD terminology, these are "faults." |
| **ECU** | Electronic Control Unit. Embedded computer in a vehicle. |
| **HARA** | Hazard Analysis and Risk Assessment (ISO 26262). Identifies safety goals and ASIL ratings. |
| **ISO-TP** | ISO 15765-2. Transport protocol for multi-frame CAN messages. Required for UDS messages exceeding 8 bytes. |
| **MDD** | Model-Driven Diagnostics. Binary database format used by CDA. Converted from ODX via the odx-converter. |
| **MISRA** | Motor Industry Software Reliability Association. MISRA C:2012 is the coding standard for safety-critical C code. |
| **NRC** | Negative Response Code. UDS error codes returned by ECUs (e.g., 0x22 = conditions not correct). |
| **NvM** | Non-Volatile Memory. Persistent storage on embedded targets for fault buffering. |
| **ODX** | Open Diagnostic Data Exchange (ASAM MCD-2D). XML-based diagnostic database format. |
| **QM** | Quality Management. ISO 26262 classification for non-safety-relevant components. No ASIL rating required. |
| **SOVD** | Service-Oriented Vehicle Diagnostics (ISO 17978). REST/HTTP API for vehicle diagnostics. |
| **UDS** | Unified Diagnostic Services (ISO 14229). Byte-level diagnostic protocol over CAN/DoIP. |

## Eclipse ecosystem

| Term | Definition |
|------|-----------|
| **ECA** | Eclipse Contributor Agreement. Must be signed before contributing to Eclipse projects. |
| **Eclipse S-CORE** | Eclipse Software-defined vehicle Core. Reference OS/middleware stack for SDV. OpenSOVD is its designated diagnostic layer. |
| **Eclipse SDV** | Eclipse Software Defined Vehicle. Umbrella project for open-source automotive software. |
| **LoLa** | Low-Latency shared-memory IPC framework in S-CORE. One of the planned fault transport backends. |
| **OpenSOVD** | Eclipse OpenSOVD. Open-source reference implementation of ISO 17978. |

## Architecture concepts

| Term | Definition |
|------|-----------|
| **Fault Library** | The API boundary between QM (SOVD) and ASIL-D (firmware) domains. Implemented as a C shim on embedded targets and a Rust crate on POSIX. |
| **FaultSink** | Trait defining fault ingestion transport. Implementations: Unix socket (default), LoLa shared-memory (S-CORE). |
| **Operation cycle** | Lifecycle concept from UDS. Faults are associated with the operation cycle during which they were detected. DFM uses this for fault aging and clear logic. |
| **SovdBackend** | Core trait for SOVD service implementations. The server routes requests to backend trait objects. |
| **SovdDb** | Trait for fault persistence. Implementations: SQLite (default), S-CORE KV (placeholder). |

## Hardware

| Term | Definition |
|------|-----------|
| **CVC** | Central Vehicle Controller. One of the physical ECUs on the test bench (STM32G474RE). |
| **FZC** | Front Zone Controller. Physical ECU on the test bench (STM32G474RE). |
| **GS_USB** | USB-to-CAN adapter using the gs_usb Linux kernel driver. |
| **RZC** | Rear Zone Controller. Physical ECU on the test bench (STM32G474RE). |
| **SC** | Safety Controller. Physical ECU on the test bench (TMS570LC43x). |
| **ST-LINK** | STMicroelectronics debug probe for STM32 microcontrollers. |
| **STM32G474RE** | ARM Cortex-M4F microcontroller (STMicroelectronics). Used for CVC, FZC, RZC ECUs. |
| **TMS570LC43x** | ARM Cortex-R5F microcontroller (Texas Instruments). Used for the safety controller. |
| **XDS110** | Texas Instruments debug probe for TMS570 microcontrollers. |
