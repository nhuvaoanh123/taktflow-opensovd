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

#![allow(clippy::doc_markdown)]

//! `cargo xtask` — workspace developer task runner.
//!
//! Currently only implements `openapi-dump`, which regenerates
//! `sovd-server/openapi.yaml` from the live `utoipa::openapi::OpenApi`
//! document in `sovd-server::openapi::ApiDoc`. The staleness gate in
//! `integration-tests/tests/phase4_openapi_staleness.rs` fails the
//! test suite whenever the committed yaml drifts from the live output,
//! so this command is the only blessed way to refresh it.

use std::{
    fs,
    path::{Path, PathBuf},
};

use cda_database::{
    datatypes::{self, DiagnosticDatabase},
    load_ecudata, update_mdd_uncompressed,
};
use cda_interfaces::datatypes::{ComParamValue, FlatbBufConfig};
use clap::{Parser, Subcommand};
use utoipa::OpenApi;

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "OpenSOVD workspace task runner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Regenerate `sovd-server/openapi.yaml` from the live ApiDoc.
    OpenapiDump {
        /// Print a diff to stderr and exit non-zero instead of
        /// overwriting the file. Used in CI staleness gates.
        #[arg(long)]
        check: bool,
    },
    /// Generate the Phase 5 CDA MDD clones used for the real Taktflow bench.
    Phase5CdaMdds {
        /// Check that the committed generated files are up to date instead
        /// of rewriting them.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::OpenapiDump { check } => match openapi_dump(check) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("xtask openapi-dump failed: {e}");
                std::process::ExitCode::FAILURE
            }
        },
        Command::Phase5CdaMdds { check } => match phase5_cda_mdds(check) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("xtask phase5-cda-mdds failed: {e}");
                std::process::ExitCode::FAILURE
            }
        },
    }
}

fn openapi_dump(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let live = sovd_server::openapi::ApiDoc::openapi().to_yaml()?;

    let path = openapi_yaml_path();
    if check {
        let committed = fs::read_to_string(&path)?;
        let live_normalised = live.replace("\r\n", "\n");
        let committed_normalised = committed.replace("\r\n", "\n");
        if live_normalised.trim() != committed_normalised.trim() {
            eprintln!("openapi.yaml is stale at {}", path.display());
            return Err("openapi.yaml staleness gate failed".into());
        }
        eprintln!("openapi.yaml is in sync");
        return Ok(());
    }

    // Ensure a trailing newline on write so git's diff view looks
    // tidy even on editors that do not auto-append one.
    let mut buf = live;
    if !buf.ends_with('\n') {
        buf.push('\n');
    }
    fs::write(&path, buf)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn openapi_yaml_path() -> PathBuf {
    workspace_root().join("sovd-server").join("openapi.yaml")
}

#[derive(Clone, Copy, Debug)]
struct Phase5MddSpec {
    remote_component_id_upper: &'static str,
    logical_address_decimal: &'static str,
}

const PHASE5_MDD_TEMPLATE_ID: &[u8; 8] = b"FLXC1000";
const PHASE5_MDD_TEMPLATE_LOGICAL_ADDRESS: &[u8; 4] = b"4096";
const PHASE5_MDD_SPECS: &[Phase5MddSpec] = &[
    Phase5MddSpec {
        remote_component_id_upper: "CVC00000",
        logical_address_decimal: "0001",
    },
    Phase5MddSpec {
        remote_component_id_upper: "FZC00000",
        logical_address_decimal: "0002",
    },
    Phase5MddSpec {
        remote_component_id_upper: "RZC00000",
        logical_address_decimal: "0003",
    },
];

fn phase5_cda_mdds(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source_mdd = phase5_template_mdd_path();
    let source_license = phase5_template_mdd_license_path();
    let output_dir = phase5_output_dir();
    let license_body = fs::read_to_string(&source_license)?;

    if !check {
        fs::create_dir_all(&output_dir)?;
    }

    for spec in PHASE5_MDD_SPECS {
        let output = output_dir.join(format!("{}.mdd", spec.remote_component_id_upper));
        let output_license =
            output_dir.join(format!("{}.mdd.license", spec.remote_component_id_upper));

        if check {
            validate_phase5_mdd(&output, spec)?;
            let committed_license = fs::read_to_string(&output_license)?;
            if committed_license != license_body {
                return Err(format!(
                    "{} is stale; rerun `cargo run -p xtask -- phase5-cda-mdds`",
                    output_license.display()
                )
                .into());
            }
        } else {
            let generated = generate_phase5_mdd(&source_mdd, spec)?;
            fs::write(&output, generated)?;
            fs::write(&output_license, &license_body)?;
            validate_phase5_mdd(&output, spec)?;
            eprintln!("wrote {}", output.display());
            eprintln!("wrote {}", output_license.display());
        }
    }

    Ok(())
}

fn generate_phase5_mdd(
    template_mdd: &Path,
    spec: &Phase5MddSpec,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tempdir = tempfile::tempdir()?;
    let temp_mdd = tempdir
        .path()
        .join(format!("{}.mdd", spec.remote_component_id_upper));
    fs::copy(template_mdd, &temp_mdd)?;
    update_mdd_uncompressed(temp_mdd.to_str().ok_or("non-UTF8 temp MDD path")?)
        .map_err(|e| format!("decompress {}: {e}", temp_mdd.display()))?;
    let mut bytes = fs::read(&temp_mdd)?;
    replace_all_ascii(
        &mut bytes,
        PHASE5_MDD_TEMPLATE_ID,
        spec.remote_component_id_upper.as_bytes(),
    )?;
    replace_all_ascii(
        &mut bytes,
        PHASE5_MDD_TEMPLATE_LOGICAL_ADDRESS,
        spec.logical_address_decimal.as_bytes(),
    )?;
    Ok(bytes)
}

fn replace_all_ascii(
    haystack: &mut [u8],
    needle: &[u8],
    replacement: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    if needle.len() != replacement.len() {
        return Err(format!(
            "replacement length mismatch for {:?} -> {:?}",
            String::from_utf8_lossy(needle),
            String::from_utf8_lossy(replacement)
        )
        .into());
    }
    if haystack.len() < needle.len() {
        return Err(format!(
            "template marker {:?} is longer than generated MDD",
            String::from_utf8_lossy(needle)
        )
        .into());
    }

    let mut replacements = 0usize;
    let limit = haystack.len().saturating_sub(needle.len());
    for start in 0..=limit {
        if &haystack[start..start + needle.len()] == needle {
            haystack[start..start + needle.len()].copy_from_slice(replacement);
            replacements += 1;
        }
    }

    if replacements == 0 {
        return Err(format!(
            "template marker {:?} not found in generated MDD",
            String::from_utf8_lossy(needle)
        )
        .into());
    }

    Ok(())
}

fn validate_phase5_mdd(
    output: &Path,
    spec: &Phase5MddSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = output
        .to_str()
        .ok_or_else(|| format!("non-UTF8 output path: {}", output.display()))?;
    let (proto_name, blob) =
        load_ecudata(path).map_err(|e| format!("load_ecudata {}: {e}", output.display()))?;
    if proto_name != spec.remote_component_id_upper {
        return Err(format!(
            "{} proto ecu_name mismatch: expected {}, got {}",
            output.display(),
            spec.remote_component_id_upper,
            proto_name
        )
        .into());
    }

    let db = DiagnosticDatabase::new_from_bytes(path.to_owned(), blob, FlatbBufConfig::default())
        .map_err(|e| format!("decode {}: {e}", output.display()))?;
    let db_name = db
        .ecu_name()
        .map_err(|e| format!("ecu_name {}: {e}", output.display()))?;
    if db_name != spec.remote_component_id_upper {
        return Err(format!(
            "{} database ecu_name mismatch: expected {}, got {}",
            output.display(),
            spec.remote_component_id_upper,
            db_name
        )
        .into());
    }

    let base = db
        .base_variant()
        .map_err(|e| format!("base_variant {}: {e}", output.display()))?;
    let Some(diag_layer) = base.diag_layer() else {
        return Err(format!("{} base variant has no diag_layer", output.display()).into());
    };
    let Some(cp_refs) = diag_layer.com_param_refs() else {
        return Err(format!("{} diag_layer has no com_param_refs", output.display()).into());
    };

    let mut saw_gateway_address = false;
    let mut saw_unique_resp_table = false;
    for cp_ref in cp_refs.iter() {
        let (name, value) =
            datatypes::resolve_comparam(&cp_ref).map_err(|e| format!("resolve_comparam: {e}"))?;
        if name == "CP_DoIPLogicalGatewayAddress" {
            let rendered = format_com_param(&value);
            if rendered == spec.logical_address_decimal {
                saw_gateway_address = true;
            }
        }
        if name == "CP_UniqueRespIdTable" {
            let rendered = format_com_param(&value);
            if rendered.contains(&format!(
                "CP_DoIPLogicalEcuAddress={}",
                spec.logical_address_decimal
            )) && rendered.contains(&format!(
                "CP_ECULayerShortName={}",
                spec.remote_component_id_upper
            )) {
                saw_unique_resp_table = true;
            }
        }
    }

    if !saw_gateway_address {
        return Err(format!(
            "{} missing CP_DoIPLogicalGatewayAddress={}",
            output.display(),
            spec.logical_address_decimal
        )
        .into());
    }
    if !saw_unique_resp_table {
        return Err(format!(
            "{} missing CP_UniqueRespIdTable alias for {} / {}",
            output.display(),
            spec.remote_component_id_upper,
            spec.logical_address_decimal
        )
        .into());
    }

    Ok(())
}

fn format_com_param(value: &ComParamValue) -> String {
    match value {
        ComParamValue::Simple(simple) => simple.value.clone(),
        ComParamValue::Complex(entries) => {
            let mut parts: Vec<String> = entries
                .iter()
                .map(|(key, value)| format!("{key}={}", format_com_param(value)))
                .collect();
            parts.sort();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

fn workspace_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent().expect("workspace root").to_path_buf()
}

fn phase5_template_mdd_path() -> PathBuf {
    workspace_root()
        .parent()
        .expect("outer workspace root")
        .join("classic-diagnostic-adapter")
        .join("testcontainer")
        .join("odx")
        .join("FLXC1000.mdd")
}

fn phase5_template_mdd_license_path() -> PathBuf {
    workspace_root()
        .parent()
        .expect("outer workspace root")
        .join("classic-diagnostic-adapter")
        .join("testcontainer")
        .join("odx")
        .join("FLXC1000.mdd.license")
}

fn phase5_output_dir() -> PathBuf {
    workspace_root().join("deploy").join("pi").join("cda-mdd")
}
