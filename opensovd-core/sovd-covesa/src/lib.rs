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

//! COVESA VSS semantic adapter scaffold for Eclipse OpenSOVD.
//!
//! ADR-0026 fixes the role of this crate: it is a thin semantic adapter
//! that loads a pinned VSS version file plus a YAML mapping table and
//! translates selected VSS paths onto existing SOVD endpoints.
//!
//! This first slice intentionally stops at contract loading. It proves:
//! - the crate has a stable home in the workspace
//! - the VSS release pin lives at `schemas/vss-version.yaml`
//! - the first mapping row is carried as data in `schemas/vss-map.yaml`
//! - Rust code can load both files together as one startup contract

use std::{fs, path::Path};

use serde::Deserialize;
use thiserror::Error;

const SCHEMAS_DIR: &str = "schemas";
const VSS_VERSION_FILE: &str = "vss-version.yaml";
const VSS_MAP_FILE: &str = "vss-map.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VssVersionPin {
    pub schema_version: u32,
    pub vss_release: String,
    pub pin_status: String,
    pub source_ref: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VssMap {
    pub schema_version: u32,
    pub vss_version: String,
    pub mappings: Vec<VssMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VssMapping {
    pub path: String,
    pub method: String,
    pub endpoint: String,
    pub direction: String,
    pub notes: String,
}

#[derive(Debug, Error)]
pub enum ContractLoadError {
    #[error("read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error(
        "mapping file vss_version `{map_version}` does not match pinned release `{pinned_version}`"
    )]
    VersionMismatch {
        pinned_version: String,
        map_version: String,
    },
    #[error("mapping catalog is empty")]
    EmptyMappings,
}

fn schema_path(file_name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(SCHEMAS_DIR)
        .join(file_name)
}

fn load_yaml<T: for<'de> Deserialize<'de>>(file_name: &str) -> Result<T, ContractLoadError> {
    let path = schema_path(file_name);
    let raw = fs::read_to_string(&path).map_err(|source| ContractLoadError::Read {
        path: path.display().to_string(),
        source,
    })?;
    serde_yaml::from_str(&raw).map_err(|source| ContractLoadError::Parse {
        path: path.display().to_string(),
        source,
    })
}

pub fn load_vss_version_pin() -> Result<VssVersionPin, ContractLoadError> {
    load_yaml(VSS_VERSION_FILE)
}

pub fn load_mapping_catalog() -> Result<VssMap, ContractLoadError> {
    load_yaml(VSS_MAP_FILE)
}

pub fn load_contracts() -> Result<(VssVersionPin, VssMap), ContractLoadError> {
    let pin = load_vss_version_pin()?;
    let map = load_mapping_catalog()?;

    if map.vss_version != pin.vss_release {
        return Err(ContractLoadError::VersionMismatch {
            pinned_version: pin.vss_release,
            map_version: map.vss_version,
        });
    }

    if map.mappings.is_empty() {
        return Err(ContractLoadError::EmptyMappings);
    }

    Ok((pin, map))
}

pub fn first_mapping_for(path: &str) -> Result<Option<VssMapping>, ContractLoadError> {
    let (_, map) = load_contracts()?;
    Ok(map
        .mappings
        .into_iter()
        .find(|mapping| mapping.path == path))
}

pub fn schemas_dir_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(SCHEMAS_DIR)
}
