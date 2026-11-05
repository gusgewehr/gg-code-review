//! Project listing grouped by namespace (group, subgroup or user).

use super::api::{api, parse_pages};
use crate::db::{RepoInput, WorkspaceInput};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct GlNamespace {
    full_path: String,
    kind: Option<String>,
}

#[derive(Deserialize)]
struct GlProject {
    path: String,
    path_with_namespace: String,
    description: Option<String>,
    default_branch: Option<String>,
    visibility: Option<String>,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    empty_repo: bool,
    namespace: GlNamespace,
}

/// Guards `repos_dir/<owner>/<name>` filesystem paths built from GitLab-supplied names.
pub(crate) fn valid_full_name(name: &str) -> bool {
    name.len() <= 256
        && name.split('/').count() >= 2
        && name.split('/').all(|seg| {
            !seg.is_empty()
                && seg != "."
                && seg != ".."
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}

type Grouped = Vec<(WorkspaceInput, Vec<RepoInput>)>;

pub(super) fn parse_projects(bytes: &[u8]) -> Result<Grouped, String> {
    let mut groups: BTreeMap<String, (WorkspaceInput, Vec<RepoInput>)> = BTreeMap::new();
    for p in parse_pages::<GlProject>(bytes)? {
        let ns = &p.namespace.full_path;
        if p.archived
            || p.empty_repo
            || !valid_full_name(&p.path_with_namespace)
            || p.path_with_namespace != format!("{ns}/{}", p.path)
        {
            continue;
        }
        let kind = if p.namespace.kind.as_deref() == Some("user") {
            "user"
        } else {
            "org"
        };
        let entry = groups.entry(ns.clone()).or_insert_with(|| {
            (
                WorkspaceInput {
                    login: ns.clone(),
                    kind: kind.into(),
                },
                Vec::new(),
            )
        });
        entry.1.push(RepoInput {
            workspace_login: ns.clone(),
            owner: ns.clone(),
            name: p.path,
            full_name: p.path_with_namespace,
            description: p.description,
            is_private: p.visibility.as_deref() != Some("public"),
            default_branch: p.default_branch,
        });
    }
    Ok(groups.into_values().collect())
}

pub(crate) fn list_all() -> Result<Grouped, String> {
    // No `simple=true`: it omits `visibility`, which the UI needs for Private/Public.
    parse_projects(&api(
        "projects?membership=true&min_access_level=30&archived=false&per_page=100",
        true,
        "projects",
    )?)
}

pub(crate) fn list_workspaces() -> Result<Vec<WorkspaceInput>, String> {
    Ok(list_all()?.into_iter().map(|(w, _)| w).collect())
}

// shortcut: one full listing per call, cache if polling many monitor-all groups gets slow
pub(crate) fn list_repos(login: &str) -> Result<Vec<RepoInput>, String> {
    Ok(list_all()?
        .into_iter()
        .find(|(w, _)| w.login == login)
        .map(|(_, r)| r)
        .unwrap_or_default())
}
