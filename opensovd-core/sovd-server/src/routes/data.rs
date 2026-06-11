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

//! Data endpoints — `/sovd/v1/components/{id}/data`.
//!
//! Mirrors the spec path table `data/data.yaml` (see
//! `docs/openapi-audit-2026-04-14.md` §5.4). Phase 5 extends the
//! original metadata list with the per-value read endpoint
//! (`GET .../data/{data-id}`) so dashboard clients can poll live DIDs
//! through the same typed boundary as CDA and demo in-memory paths.
//! PROD-12 adds the spec data filters (`categories` / `groups` /
//! `tags`) on the list endpoint.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use axum_extra::extract::Query;
use serde::Deserialize;
use sovd_interfaces::{
    ComponentId,
    spec::data::{Datas, ReadValue, ValueMetadata},
};
use utoipa::IntoParams;

use crate::{InMemoryServer, routes::error::ApiError};

/// Query filters for [`list_data`].
///
/// All three parameters are repeatable (`?groups=a&groups=b`), which is
/// why this uses `axum_extra`'s `Query` instead of `axum`'s (the latter
/// rejects repeated keys for `Vec` fields).
///
/// Provenance: spec `data/data.yaml` list parameters. The spec makes
/// `groups` and `categories` mutually exclusive scopes: `groups` takes
/// precedence and `categories` is ignored when both are supplied, while
/// `tags` is AND-combined with the chosen scope. Semantics mirror
/// upstream opensovd-core (`ae2a141` repeated-param fix and the
/// fix/data-filter scope rule).
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DataListQuery {
    /// Keep values whose `category` matches any listed category.
    /// Ignored when `groups` is also supplied.
    #[serde(default)]
    pub categories: Vec<String>,

    /// Keep values belonging to any listed group. Takes precedence
    /// over `categories`.
    #[serde(default)]
    pub groups: Vec<String>,

    /// Keep values carrying at least one listed tag; combined with the
    /// scope filter.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Mutually exclusive list scope resolved from a [`DataListQuery`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum DataScope {
    /// Match values that belong to any of the listed groups.
    Groups(Vec<String>),
    /// Match values whose category is any of the listed categories.
    Categories(Vec<String>),
}

/// Resolve the request scope: `groups` wins, `categories` is ignored
/// when both are present, and an unfiltered request has no scope.
fn resolve_scope(query: &DataListQuery) -> Option<DataScope> {
    if !query.groups.is_empty() {
        Some(DataScope::Groups(query.groups.clone()))
    } else if !query.categories.is_empty() {
        Some(DataScope::Categories(query.categories.clone()))
    } else {
        None
    }
}

/// `true` when `item` survives both the scope and the tag filter.
fn matches_filter(item: &ValueMetadata, scope: Option<&DataScope>, tags: &[String]) -> bool {
    let scope_ok = match scope {
        Some(DataScope::Groups(groups)) => item
            .groups
            .as_deref()
            .is_some_and(|item_groups| groups.iter().any(|g| item_groups.contains(g))),
        Some(DataScope::Categories(categories)) => categories.contains(&item.category),
        None => true,
    };
    scope_ok
        && (tags.is_empty()
            || item
                .tags
                .as_deref()
                .is_some_and(|item_tags| tags.iter().any(|t| item_tags.contains(t))))
}

/// `GET /sovd/v1/components/{component_id}/data` — list the
/// data-metadata catalog, optionally filtered by category, group, or
/// tags.
///
/// # Errors
///
/// Returns 404 if the component is unknown; other
/// [`SovdError`](sovd_interfaces::SovdError) values are mapped via
/// [`ApiError`].
#[utoipa::path(
    get,
    path = "/sovd/v1/components/{component_id}/data",
    operation_id = "listData",
    tag = "data-access",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
        DataListQuery,
    ),
    responses(
        (status = 200, description = "Data-metadata catalog", body = Datas),
        (status = 404, description = "Component not found"),
    ),
)]
pub async fn list_data(
    State(server): State<Arc<InMemoryServer>>,
    Path(component_id): Path<String>,
    Query(query): Query<DataListQuery>,
) -> Result<Json<Datas>, ApiError> {
    let component = ComponentId::new(component_id);
    let mut datas = server.dispatch_list_data(&component).await?;
    let scope = resolve_scope(&query);
    if scope.is_some() || !query.tags.is_empty() {
        datas
            .items
            .retain(|item| matches_filter(item, scope.as_ref(), &query.tags));
    }
    Ok(Json(datas))
}

/// `GET /sovd/v1/components/{component_id}/data/{data_id}` — read one
/// live data value.
///
/// # Errors
///
/// Returns 404 if the component or data id is unknown; other
/// [`SovdError`](sovd_interfaces::SovdError) values are mapped via
/// [`ApiError`].
#[utoipa::path(
    get,
    path = "/sovd/v1/components/{component_id}/data/{data_id}",
    operation_id = "readData",
    tag = "data-access",
    params(
        ("component_id" = String, Path, description = "Stable component identifier"),
        ("data_id" = String, Path, description = "Stable data identifier"),
    ),
    responses(
        (status = 200, description = "Live data value", body = ReadValue),
        (status = 404, description = "Component or data id not found"),
    ),
)]
pub async fn read_data(
    State(server): State<Arc<InMemoryServer>>,
    Path((component_id, data_id)): Path<(String, String)>,
) -> Result<Json<ReadValue>, ApiError> {
    let component = ComponentId::new(component_id);
    Ok(Json(server.dispatch_read_data(&component, &data_id).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(
        id: &str,
        category: &str,
        groups: Option<&[&str]>,
        tags: Option<&[&str]>,
    ) -> ValueMetadata {
        ValueMetadata {
            id: id.to_owned(),
            name: id.to_owned(),
            translation_id: None,
            category: category.to_owned(),
            groups: groups.map(|g| g.iter().map(|s| (*s).to_owned()).collect()),
            tags: tags.map(|t| t.iter().map(|s| (*s).to_owned()).collect()),
        }
    }

    fn query(categories: &[&str], groups: &[&str], tags: &[&str]) -> DataListQuery {
        DataListQuery {
            categories: categories.iter().map(|s| (*s).to_owned()).collect(),
            groups: groups.iter().map(|s| (*s).to_owned()).collect(),
            tags: tags.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn scope_is_none_without_filters() {
        assert_eq!(resolve_scope(&query(&[], &[], &[])), None);
        // A tags-only query also has no scope.
        assert_eq!(resolve_scope(&query(&[], &[], &["live"])), None);
    }

    #[test]
    fn groups_take_precedence_over_categories() {
        let scope = resolve_scope(&query(&["identData"], &["powertrain"], &[]));
        assert_eq!(scope, Some(DataScope::Groups(vec!["powertrain".to_owned()])));
    }

    #[test]
    fn categories_scope_matches_by_category_union() {
        let scope = resolve_scope(&query(&["identData", "sysInfo"], &[], &[])).unwrap();
        assert!(matches_filter(&meta("vin", "identData", None, None), Some(&scope), &[]));
        assert!(matches_filter(&meta("ecu", "sysInfo", None, None), Some(&scope), &[]));
        assert!(!matches_filter(&meta("rpm", "currentData", None, None), Some(&scope), &[]));
    }

    #[test]
    fn groups_scope_matches_any_listed_group() {
        let scope = resolve_scope(&query(&[], &["powertrain", "body"], &[])).unwrap();
        let in_body = meta("win", "currentData", Some(&["body"]), None);
        let in_other = meta("rpm", "currentData", Some(&["chassis"]), None);
        let no_groups = meta("vin", "identData", None, None);
        assert!(matches_filter(&in_body, Some(&scope), &[]));
        assert!(!matches_filter(&in_other, Some(&scope), &[]));
        assert!(!matches_filter(&no_groups, Some(&scope), &[]));
    }

    #[test]
    fn tags_are_and_combined_with_scope() {
        let scope = resolve_scope(&query(&["currentData"], &[], &[])).unwrap();
        let tagged = meta("rpm", "currentData", None, Some(&["live"]));
        let untagged = meta("temp", "currentData", None, None);
        let wrong_category = meta("vin", "identData", None, Some(&["live"]));
        let tags = vec!["live".to_owned()];
        assert!(matches_filter(&tagged, Some(&scope), &tags));
        assert!(!matches_filter(&untagged, Some(&scope), &tags));
        assert!(!matches_filter(&wrong_category, Some(&scope), &tags));
    }

    #[tokio::test]
    async fn list_data_filters_demo_catalog_by_category() {
        let server = Arc::new(InMemoryServer::new_with_demo_data());
        let all = list_data(
            State(Arc::clone(&server)),
            Path("cvc".to_owned()),
            Query(DataListQuery::default()),
        )
        .await
        .expect("unfiltered list");
        assert!(!all.0.items.is_empty(), "demo catalog must not be empty");

        let ident = list_data(
            State(Arc::clone(&server)),
            Path("cvc".to_owned()),
            Query(query(&["identData"], &[], &[])),
        )
        .await
        .expect("category-filtered list");
        assert!(ident.0.items.iter().all(|i| i.category == "identData"));

        let none = list_data(
            State(server),
            Path("cvc".to_owned()),
            Query(query(&["x-no-such-category"], &[], &[])),
        )
        .await
        .expect("empty-filtered list");
        assert!(none.0.items.is_empty());
    }
}
