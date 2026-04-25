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

//! Runtime MDD loading and service resolution.

use std::{collections::HashMap, sync::Arc};

use cda_core::EcuManager as CdaEcuManager;
use cda_database::{datatypes::DiagnosticDatabase, load_ecudata};
use cda_interfaces::{
    DiagComm, DiagServiceError, EcuAddressProvider as _, EcuManager as _, EcuManagerType,
    ParameterTypeMetadata, Protocol, ServicePayload, diagservices::DiagServiceResponse as _,
    service_ids,
};
use cda_plugin_security::DefaultSecurityPluginData;
use serde_json::Value;
use thiserror::Error;

use crate::config::{Configuration, TargetConfig};

type RuntimeEcuManager = CdaEcuManager<DefaultSecurityPluginData>;

#[derive(Debug, Error)]
pub enum MddRegistryError {
    #[error("failed to load MDD `{path}`: {message}")]
    LoadMdd { path: String, message: String },
    #[error("invalid diagnostic database `{path}`: {source}")]
    InvalidDatabase {
        path: String,
        source: DiagServiceError,
    },
    #[error(
        "target `{component_id}` logical address mismatch: config has {configured:#06x}, MDD resolved {resolved:#06x}"
    )]
    LogicalAddressMismatch {
        component_id: String,
        configured: u16,
        resolved: u16,
    },
    #[error("duplicate logical address {logical_address:#06x} for component `{component_id}`")]
    DuplicateLogicalAddress {
        logical_address: u16,
        component_id: String,
    },
    #[error("invalid DID route key `{key}` for component `{component_id}`")]
    InvalidDidRouteKey { component_id: String, key: String },
    #[error("invalid routine route key `{key}` for component `{component_id}`")]
    InvalidRoutineRouteKey { component_id: String, key: String },
}

#[derive(Clone)]
pub struct MddRegistry {
    targets: HashMap<u16, Arc<LoadedTarget>>,
}

#[derive(Clone)]
pub struct LoadedTarget {
    pub component_id: String,
    pub ecu_name: String,
    pub logical_address: u16,
    pub naming_convention: cda_interfaces::datatypes::DatabaseNamingConvention,
    pub fault_scope: Option<String>,
    pub manager: Arc<RuntimeEcuManager>,
    did_routes: HashMap<u16, String>,
    routine_routes: HashMap<u16, String>,
}

#[derive(Clone)]
pub struct ResolvedService {
    pub diag_comm: DiagComm,
    pub dynamic_parameters: Option<Value>,
}

impl MddRegistry {
    pub fn load(config: &Configuration) -> Result<Self, MddRegistryError> {
        let mut targets = HashMap::new();

        for target in &config.targets {
            let loaded = Arc::new(LoadedTarget::load(target)?);
            if targets
                .insert(loaded.logical_address, Arc::clone(&loaded))
                .is_some()
            {
                return Err(MddRegistryError::DuplicateLogicalAddress {
                    logical_address: loaded.logical_address,
                    component_id: loaded.component_id.clone(),
                });
            }
        }

        Ok(Self { targets })
    }

    #[must_use]
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    #[must_use]
    pub fn resolve(&self, logical_address: u16) -> Option<Arc<LoadedTarget>> {
        self.targets.get(&logical_address).cloned()
    }
}

impl LoadedTarget {
    fn load(config: &TargetConfig) -> Result<Self, MddRegistryError> {
        let (_ecu_name, blob) =
            load_ecudata(&config.mdd_path).map_err(|source| MddRegistryError::LoadMdd {
                path: config.mdd_path.clone(),
                message: source.to_string(),
            })?;
        let database = DiagnosticDatabase::new_from_bytes(
            config.mdd_path.clone(),
            blob,
            cda_interfaces::datatypes::FlatbBufConfig::default(),
        )
        .map_err(|source| MddRegistryError::InvalidDatabase {
            path: config.mdd_path.clone(),
            source,
        })?;

        let manager = RuntimeEcuManager::new(
            database,
            Protocol::DoIp,
            &config.com_params,
            config.database_naming_convention.clone(),
            EcuManagerType::Ecu,
            &config.functional_description,
            config.fallback_to_base_variant,
        )
        .map_err(|source| MddRegistryError::InvalidDatabase {
            path: config.mdd_path.clone(),
            source,
        })?;

        let resolved_logical_address = manager.logical_address();
        if let Some(configured) = config.logical_address
            && configured != resolved_logical_address
        {
            return Err(MddRegistryError::LogicalAddressMismatch {
                component_id: config.component_id.clone(),
                configured,
                resolved: resolved_logical_address,
            });
        }

        Ok(Self {
            component_id: config.component_id.clone(),
            ecu_name: manager.ecu_name(),
            logical_address: resolved_logical_address,
            naming_convention: config.database_naming_convention.clone(),
            fault_scope: config.fault_scope.clone(),
            manager: Arc::new(manager),
            did_routes: parse_routes(&config.did_routes, &config.component_id, true)?,
            routine_routes: parse_routes(&config.routine_routes, &config.component_id, false)?,
        })
    }

    pub async fn resolve_service(
        &self,
        request: &[u8],
        source_address: u16,
    ) -> Result<ResolvedService, DiagServiceError> {
        let mut matches = self.manager.lookup_diagcomms_by_request_prefix(request)?;
        let diag_comm = matches
            .drain(..)
            .next()
            .ok_or_else(|| DiagServiceError::NotFound("No matching service".to_owned()))?;

        let payload = ServicePayload {
            data: request.to_vec(),
            source_address,
            target_address: self.logical_address,
            new_session: None,
            new_security: None,
        };
        let parsed = self
            .manager
            .convert_request_from_uds(&diag_comm, &payload, true)
            .await?;
        let dynamic_parameters = filter_dynamic_parameters(
            parsed.into_json()?.data,
            self.manager
                .get_service_parameter_metadata(&diag_comm.name)?,
        );

        Ok(ResolvedService {
            diag_comm,
            dynamic_parameters,
        })
    }

    pub fn data_route_id(&self, did: u16, service_name: &str) -> String {
        self.did_routes.get(&did).cloned().unwrap_or_else(|| {
            self.fallback_route_id(service_ids::READ_DATA_BY_IDENTIFIER, service_name)
        })
    }

    pub fn explicit_data_route_id(&self, did: u16) -> Option<String> {
        self.did_routes.get(&did).cloned()
    }

    pub fn operation_route_id(&self, routine_id: u16, service_name: &str) -> String {
        self.routine_routes
            .get(&routine_id)
            .cloned()
            .unwrap_or_else(|| self.fallback_route_id(service_ids::ROUTINE_CONTROL, service_name))
    }

    pub fn explicit_operation_route_id(&self, routine_id: u16) -> Option<String> {
        self.routine_routes.get(&routine_id).cloned()
    }

    pub fn dtc_scope_for(
        &self,
        subfunction: cda_interfaces::datatypes::DtcReadInformationFunction,
    ) -> Result<Option<String>, DiagServiceError> {
        let lookups = self.manager.lookup_dtc_services(vec![subfunction])?;
        Ok(lookups
            .get(&subfunction)
            .map(|lookup| lookup.scope.clone())
            .or_else(|| self.fault_scope.clone()))
    }

    fn fallback_route_id(&self, sid: u8, service_name: &str) -> String {
        let without_service_affix = self
            .naming_convention
            .trim_service_name_affixes(sid, service_name.to_owned());
        self.naming_convention
            .trim_short_name_affixes(&without_service_affix)
            .to_lowercase()
    }
}

fn parse_routes(
    raw: &HashMap<String, String>,
    component_id: &str,
    did_routes: bool,
) -> Result<HashMap<u16, String>, MddRegistryError> {
    raw.iter()
        .map(|(key, value)| {
            let parsed = parse_u16_key(key).ok_or_else(|| {
                if did_routes {
                    MddRegistryError::InvalidDidRouteKey {
                        component_id: component_id.to_owned(),
                        key: key.clone(),
                    }
                } else {
                    MddRegistryError::InvalidRoutineRouteKey {
                        component_id: component_id.to_owned(),
                        key: key.clone(),
                    }
                }
            })?;
            Ok((parsed, value.clone()))
        })
        .collect()
}

fn parse_u16_key(value: &str) -> Option<u16> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u16::from_str_radix(hex, 16).ok();
    }
    trimmed.parse::<u16>().ok()
}

fn filter_dynamic_parameters(
    value: Value,
    metadata: Vec<cda_interfaces::ServiceParameterMetadata>,
) -> Option<Value> {
    let Value::Object(map) = value else {
        return None;
    };

    let dynamic_names = metadata
        .into_iter()
        .filter_map(|metadata| match metadata.param_type {
            ParameterTypeMetadata::Value => Some(metadata.name),
            ParameterTypeMetadata::CodedConst { .. } | ParameterTypeMetadata::PhysConst { .. } => {
                None
            }
        })
        .collect::<Vec<_>>();

    if dynamic_names.is_empty() {
        return None;
    }

    let filtered = dynamic_names
        .into_iter()
        .filter_map(|name| map.get(&name).map(|value| (name, value.clone())))
        .collect::<serde_json::Map<_, _>>();

    if filtered.is_empty() {
        None
    } else {
        Some(Value::Object(filtered))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parses_hex_or_decimal_route_keys() {
        assert_eq!(parse_u16_key("0xF190"), Some(0xF190));
        assert_eq!(parse_u16_key("61840"), Some(0xF190));
        assert_eq!(parse_u16_key("garbage"), None);
    }

    #[test]
    fn filters_out_constant_parameters() {
        let json = serde_json::json!({
            "SID": 0x31,
            "RC_SUBFUNC": 0x01,
            "ROUTINE_ID": 0x0246,
            "argument": [0x12, 0x34],
        });
        let metadata = vec![
            cda_interfaces::ServiceParameterMetadata {
                name: "SID".to_owned(),
                semantic: None,
                param_type: ParameterTypeMetadata::CodedConst {
                    coded_value: "0x31".to_owned(),
                },
            },
            cda_interfaces::ServiceParameterMetadata {
                name: "argument".to_owned(),
                semantic: None,
                param_type: ParameterTypeMetadata::Value,
            },
        ];

        let filtered = filter_dynamic_parameters(json, metadata).unwrap();
        assert_eq!(filtered, serde_json::json!({ "argument": [0x12, 0x34] }));
    }
}
