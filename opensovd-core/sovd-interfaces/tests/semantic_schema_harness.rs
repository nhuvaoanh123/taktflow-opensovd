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

//! Validation harness for semantic schema files.
//!
//! The upstream Phase 2 semantic layer stores schema files under
//! `schemas/semantic/`. This test walks every `*.schema.yaml` file in that
//! directory and checks a small, generic JSON Schema contract so future domain
//! schemas can be added without changing the harness shape.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

fn semantic_schemas_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas")
        .join("semantic")
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonSchemaDocument {
    #[serde(rename = "$schema")]
    draft: String,
    #[serde(rename = "$id")]
    id: String,
    title: String,
    description: String,
    #[serde(rename = "type")]
    schema_type: String,
    #[serde(rename = "additionalProperties")]
    additional_properties: bool,
    required: Vec<String>,
    properties: BTreeMap<String, JsonSchemaProperty>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonSchemaProperty {
    #[serde(rename = "type")]
    property_type: String,
    description: Option<String>,
    minimum: Option<u64>,
    #[serde(rename = "minLength")]
    min_length: Option<u64>,
    #[serde(rename = "minItems")]
    min_items: Option<u64>,
    pattern: Option<String>,
    #[serde(rename = "enum")]
    enum_values: Option<Vec<String>>,
    items: Option<Box<JsonSchemaProperty>>,
    required: Option<Vec<String>>,
    properties: Option<BTreeMap<String, JsonSchemaProperty>>,
    #[serde(rename = "additionalProperties")]
    additional_properties: Option<bool>,
}

fn schema_files() -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(semantic_schemas_dir())
        .expect("read semantic schema dir")
        .map(|entry| entry.expect("read schema dir entry").path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".schema.yaml"))
        })
        .collect();
    files.sort();
    files
}

fn validate_property(name: &str, property: &JsonSchemaProperty) {
    assert!(
        !property.property_type.is_empty(),
        "property {name} must declare a type"
    );

    if let Some(values) = &property.enum_values {
        assert!(!values.is_empty(), "property {name} enum must not be empty");
    }

    if let Some(min_items) = property.min_items {
        assert!(
            property.property_type == "array",
            "property {name} uses minItems but is not an array"
        );
        assert!(min_items >= 1, "property {name} minItems must be >= 1");
    }

    if let Some(min_length) = property.min_length {
        assert!(
            property.property_type == "string",
            "property {name} uses minLength but is not a string"
        );
        assert!(min_length >= 1, "property {name} minLength must be >= 1");
    }

    if let Some(minimum) = property.minimum {
        assert!(
            property.property_type == "integer",
            "property {name} uses minimum but is not an integer"
        );
        assert!(minimum >= 1, "property {name} minimum must be >= 1");
    }

    if property.pattern.is_some() {
        assert!(
            property.property_type == "string",
            "property {name} uses pattern but is not a string"
        );
    }

    if let Some(items) = &property.items {
        assert!(
            property.property_type == "array",
            "property {name} has items but is not an array"
        );
        validate_property(&format!("{name}[]"), items);
    }

    if let Some(required) = &property.required {
        let nested = property.properties.as_ref().unwrap_or_else(|| {
            panic!("property {name} has required fields but no nested properties")
        });
        for nested_name in required {
            assert!(
                nested.contains_key(nested_name),
                "property {name} requires nested key {nested_name} that is not declared"
            );
        }
    }

    if let Some(nested) = &property.properties {
        assert!(
            property.property_type == "object",
            "property {name} declares nested properties but is not an object"
        );
        for (nested_name, nested_property) in nested {
            validate_property(&format!("{name}.{nested_name}"), nested_property);
        }
    }
}

#[test]
fn semantic_schema_files_validate() {
    let files = schema_files();
    assert!(
        !files.is_empty(),
        "expected at least one semantic schema file under {}",
        semantic_schemas_dir().display()
    );

    for path in files {
        let display = path.display();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read schema {display}: {error}"));
        let schema: JsonSchemaDocument = serde_yaml::from_str(&raw)
            .unwrap_or_else(|error| panic!("parse schema {display}: {error}"));

        assert_eq!(
            schema.draft, "https://json-schema.org/draft/2020-12/schema",
            "schema {display} must pin the JSON Schema draft"
        );
        assert!(
            schema.id.contains("/schemas/semantic/"),
            "schema {display} id must live under /schemas/semantic/"
        );
        assert!(
            !schema.title.is_empty() && !schema.description.is_empty(),
            "schema {display} must declare title and description"
        );
        assert_eq!(
            schema.schema_type, "object",
            "schema {display} root type must be object"
        );
        assert!(
            !schema.additional_properties,
            "schema {display} must explicitly close over unknown top-level fields"
        );
        assert!(
            !schema.required.is_empty(),
            "schema {display} must declare at least one required field"
        );

        for required_name in &schema.required {
            assert!(
                schema.properties.contains_key(required_name),
                "schema {display} requires field {required_name} that is not declared"
            );
        }

        for (name, property) in &schema.properties {
            if property.additional_properties.is_some() {
                assert_eq!(
                    property.property_type, "object",
                    "property {name} only uses additionalProperties on object types"
                );
            }
            validate_property(name, property);
        }
    }
}
