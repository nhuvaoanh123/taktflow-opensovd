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

//! UDS request parsing and reply encoding for the DoIP-first proxy path.

use thiserror::Error;

pub const SID_CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
pub const SID_READ_DTC_INFORMATION: u8 = 0x19;
pub const SID_READ_DATA_BY_IDENTIFIER: u8 = 0x22;
pub const SID_WRITE_DATA_BY_IDENTIFIER: u8 = 0x2E;
pub const SID_AUTHENTICATION: u8 = 0x29;
pub const SID_SECURITY_ACCESS: u8 = 0x27;
pub const SID_DIAGNOSTIC_SESSION_CONTROL: u8 = 0x10;
pub const SID_ROUTINE_CONTROL: u8 = 0x31;

pub const NRC_GENERAL_REJECT: u8 = 0x10;
pub const NRC_SERVICE_NOT_SUPPORTED: u8 = 0x11;
pub const NRC_SUBFUNCTION_NOT_SUPPORTED: u8 = 0x12;
pub const NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT: u8 = 0x13;
pub const NRC_BUSY_REPEAT_REQUEST: u8 = 0x21;
pub const NRC_CONDITIONS_NOT_CORRECT: u8 = 0x22;
pub const NRC_NO_RESPONSE_FROM_SUBNET_COMPONENT: u8 = 0x25;
pub const NRC_REQUEST_OUT_OF_RANGE: u8 = 0x31;
pub const NRC_SECURITY_ACCESS_DENIED: u8 = 0x33;
pub const NRC_RESPONSE_PENDING: u8 = 0x78;

pub const ROUTINE_SUBFUNCTION_START: u8 = 0x01;
pub const ROUTINE_SUBFUNCTION_STOP: u8 = 0x02;
pub const ROUTINE_SUBFUNCTION_RESULTS: u8 = 0x03;

pub const DTC_SUBFUNCTION_REPORT_COUNT_BY_STATUS_MASK: u8 = 0x01;
pub const DTC_SUBFUNCTION_REPORT_BY_STATUS_MASK: u8 = 0x02;
pub const DTC_FORMAT_IDENTIFIER_ISO_14229_1: u8 = 0x01;
pub const DTC_STATUS_AVAILABILITY_MASK_ALL: u8 = 0xFF;
pub const ALL_DTC_GROUP: [u8; 3] = [0xFF, 0xFF, 0xFF];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticRequest {
    ReadDataByIdentifier { did: u16, raw: Vec<u8> },
    RoutineStart { routine_id: u16, raw: Vec<u8> },
    RoutineResults { routine_id: u16, raw: Vec<u8> },
    ReadDtcCountByStatusMask { status_mask: u8 },
    ReadDtcByStatusMask { status_mask: u8 },
    ClearAllDiagnosticInformation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("UDS negative response for service {request_sid:#04x}: {code:#04x}")]
pub struct NegativeResponse {
    pub request_sid: u8,
    pub code: u8,
}

impl NegativeResponse {
    #[must_use]
    pub const fn new(request_sid: u8, code: u8) -> Self {
        Self { request_sid, code }
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        vec![0x7F, self.request_sid, self.code]
    }
}

#[derive(Debug, Error)]
pub enum UdsEncodingError {
    #[error("value cannot be encoded as UDS bytes")]
    UnsupportedValueShape,
    #[error("DTC count {count} exceeds UDS 16-bit range")]
    DtcCountTooLarge { count: usize },
}

#[derive(Debug, Default, Clone)]
pub struct IsoTpReassembler;

impl IsoTpReassembler {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The first cut is DoIP-only, so each diagnostic message already carries
    /// one complete UDS PDU. The reassembler still exists to keep the north-
    /// face shape ready for a future CAN-TP tether path.
    pub fn push_complete_pdu(&mut self, payload: &[u8]) -> Result<Vec<u8>, NegativeResponse> {
        if payload.is_empty() {
            return Err(NegativeResponse::new(
                0x00,
                NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
            ));
        }
        Ok(payload.to_vec())
    }
}

pub fn parse_request(payload: &[u8]) -> Result<DiagnosticRequest, NegativeResponse> {
    let Some(&sid) = payload.first() else {
        return Err(NegativeResponse::new(
            0x00,
            NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
        ));
    };

    match sid {
        SID_READ_DATA_BY_IDENTIFIER => {
            if payload.len() != 3 {
                return Err(NegativeResponse::new(
                    sid,
                    NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
                ));
            }
            Ok(DiagnosticRequest::ReadDataByIdentifier {
                did: u16::from_be_bytes([payload[1], payload[2]]),
                raw: payload.to_vec(),
            })
        }
        SID_ROUTINE_CONTROL => {
            if payload.len() < 4 {
                return Err(NegativeResponse::new(
                    sid,
                    NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
                ));
            }
            let subfunction = payload[1];
            let routine_id = u16::from_be_bytes([payload[2], payload[3]]);
            match subfunction {
                ROUTINE_SUBFUNCTION_START => Ok(DiagnosticRequest::RoutineStart {
                    routine_id,
                    raw: payload.to_vec(),
                }),
                ROUTINE_SUBFUNCTION_RESULTS => Ok(DiagnosticRequest::RoutineResults {
                    routine_id,
                    raw: payload.to_vec(),
                }),
                ROUTINE_SUBFUNCTION_STOP => {
                    Err(NegativeResponse::new(sid, NRC_SUBFUNCTION_NOT_SUPPORTED))
                }
                _ => Err(NegativeResponse::new(sid, NRC_SUBFUNCTION_NOT_SUPPORTED)),
            }
        }
        SID_READ_DTC_INFORMATION => {
            if payload.len() != 3 {
                return Err(NegativeResponse::new(
                    sid,
                    NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
                ));
            }
            match payload[1] {
                DTC_SUBFUNCTION_REPORT_COUNT_BY_STATUS_MASK => {
                    Ok(DiagnosticRequest::ReadDtcCountByStatusMask {
                        status_mask: payload[2],
                    })
                }
                DTC_SUBFUNCTION_REPORT_BY_STATUS_MASK => {
                    Ok(DiagnosticRequest::ReadDtcByStatusMask {
                        status_mask: payload[2],
                    })
                }
                _ => Err(NegativeResponse::new(sid, NRC_SUBFUNCTION_NOT_SUPPORTED)),
            }
        }
        SID_CLEAR_DIAGNOSTIC_INFORMATION => {
            if payload.len() != 4 {
                return Err(NegativeResponse::new(
                    sid,
                    NRC_INCORRECT_MESSAGE_LENGTH_OR_INVALID_FORMAT,
                ));
            }
            if payload[1..4] != ALL_DTC_GROUP {
                return Err(NegativeResponse::new(sid, NRC_REQUEST_OUT_OF_RANGE));
            }
            Ok(DiagnosticRequest::ClearAllDiagnosticInformation)
        }
        SID_WRITE_DATA_BY_IDENTIFIER
        | SID_DIAGNOSTIC_SESSION_CONTROL
        | SID_SECURITY_ACCESS
        | SID_AUTHENTICATION => Err(NegativeResponse::new(sid, NRC_SERVICE_NOT_SUPPORTED)),
        _ => Err(NegativeResponse::new(sid, NRC_SERVICE_NOT_SUPPORTED)),
    }
}

#[must_use]
pub fn positive_read_data_by_identifier(did: u16, data: &[u8]) -> Vec<u8> {
    let mut response = Vec::with_capacity(3usize.saturating_add(data.len()));
    response.push(SID_READ_DATA_BY_IDENTIFIER.saturating_add(0x40));
    response.extend_from_slice(&did.to_be_bytes());
    response.extend_from_slice(data);
    response
}

#[must_use]
pub fn positive_routine_control(subfunction: u8, routine_id: u16, data: &[u8]) -> Vec<u8> {
    let mut response = Vec::with_capacity(4usize.saturating_add(data.len()));
    response.push(SID_ROUTINE_CONTROL.saturating_add(0x40));
    response.push(subfunction);
    response.extend_from_slice(&routine_id.to_be_bytes());
    response.extend_from_slice(data);
    response
}

pub fn positive_dtc_count_by_status_mask(count: usize) -> Result<Vec<u8>, UdsEncodingError> {
    let count = u16::try_from(count).map_err(|_| UdsEncodingError::DtcCountTooLarge { count })?;
    let [count_hi, count_lo] = count.to_be_bytes();
    Ok(vec![
        SID_READ_DTC_INFORMATION.saturating_add(0x40),
        DTC_SUBFUNCTION_REPORT_COUNT_BY_STATUS_MASK,
        DTC_STATUS_AVAILABILITY_MASK_ALL,
        DTC_FORMAT_IDENTIFIER_ISO_14229_1,
        count_hi,
        count_lo,
    ])
}

#[must_use]
pub fn positive_dtc_list_by_status_mask(records: &[(u32, u8)]) -> Vec<u8> {
    let mut response = Vec::with_capacity(3usize.saturating_add(records.len().saturating_mul(4)));
    response.push(SID_READ_DTC_INFORMATION.saturating_add(0x40));
    response.push(DTC_SUBFUNCTION_REPORT_BY_STATUS_MASK);
    response.push(DTC_STATUS_AVAILABILITY_MASK_ALL);
    for (dtc, status) in records {
        let dtc_bytes = dtc.to_be_bytes();
        response.extend_from_slice(&dtc_bytes[1..4]);
        response.push(*status);
    }
    response
}

#[must_use]
pub fn positive_clear_all_diagnostic_information() -> Vec<u8> {
    vec![SID_CLEAR_DIAGNOSTIC_INFORMATION.saturating_add(0x40)]
}

#[must_use]
pub fn response_pending(request_sid: u8) -> Vec<u8> {
    NegativeResponse::new(request_sid, NRC_RESPONSE_PENDING).into_bytes()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parses_read_data_request() {
        let request = parse_request(&[SID_READ_DATA_BY_IDENTIFIER, 0xF1, 0x90]).unwrap();
        assert_eq!(
            request,
            DiagnosticRequest::ReadDataByIdentifier {
                did: 0xF190,
                raw: vec![SID_READ_DATA_BY_IDENTIFIER, 0xF1, 0x90],
            }
        );
    }

    #[test]
    fn denies_stop_routine() {
        let err = parse_request(&[SID_ROUTINE_CONTROL, ROUTINE_SUBFUNCTION_STOP, 0x12, 0x34])
            .unwrap_err();
        assert_eq!(
            err,
            NegativeResponse::new(SID_ROUTINE_CONTROL, NRC_SUBFUNCTION_NOT_SUPPORTED)
        );
    }

    #[test]
    fn encodes_dtc_list() {
        let payload = positive_dtc_list_by_status_mask(&[(0x12_34_56, 0x09)]);
        assert_eq!(payload, vec![0x59, 0x02, 0xFF, 0x12, 0x34, 0x56, 0x09]);
    }

    #[test]
    fn isotp_reassembler_passthroughs_doip_pdus() {
        let mut reassembler = IsoTpReassembler::new();
        let pdu = reassembler
            .push_complete_pdu(&[SID_READ_DATA_BY_IDENTIFIER, 0xF1, 0x90])
            .unwrap();
        assert_eq!(pdu, vec![SID_READ_DATA_BY_IDENTIFIER, 0xF1, 0x90]);
    }
}
