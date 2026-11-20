use bytes::Bytes;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::error::EngineError;

/// Domain-specific Modrinth project categories exposed by Oxirinth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainKind {
    Mod,
    Plugin,
    ResourcePack,
    DataPack,
    Shader,
    Modpack,
    Server,
}

impl DomainKind {
    /// All seven supported Modrinth project domains.
    pub const ALL: [Self; 7] = [
        Self::Mod,
        Self::Plugin,
        Self::ResourcePack,
        Self::DataPack,
        Self::Shader,
        Self::Modpack,
        Self::Server,
    ];

    /// Return the canonical Modrinth v3 `project_types` token.
    pub const fn as_project_type(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Plugin => "plugin",
            Self::ResourcePack => "resourcepack",
            Self::DataPack => "datapack",
            Self::Shader => "shader",
            Self::Modpack => "modpack",
            Self::Server => "minecraft_java_server",
        }
    }

    /// Return the injected Modrinth `/v3/search` facet token.
    pub const fn as_facet(self) -> &'static str {
        match self {
            Self::Mod => "project_types:mod",
            Self::Plugin => "project_types:plugin",
            Self::ResourcePack => "project_types:resourcepack",
            Self::DataPack => "project_types:datapack",
            Self::Shader => "project_types:shader",
            Self::Modpack => "project_types:modpack",
            Self::Server => "project_types:minecraft_java_server",
        }
    }

    /// Return the loader facet key used on `/v3/search` for this domain.
    pub const fn loader_facet_prefix(self) -> &'static str {
        match self {
            Self::Modpack | Self::Server => "mrpack_loaders",
            Self::Mod | Self::Plugin | Self::ResourcePack | Self::DataPack | Self::Shader => {
                "loaders"
            }
        }
    }

    /// Parse a domain identifier string (case-insensitive).
    pub fn from_name(name: &str) -> Result<Self, EngineError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "mod" => Ok(Self::Mod),
            "plugin" => Ok(Self::Plugin),
            "resourcepack" | "resource_pack" => Ok(Self::ResourcePack),
            "datapack" | "data_pack" => Ok(Self::DataPack),
            "shader" => Ok(Self::Shader),
            "modpack" => Ok(Self::Modpack),
            "server" | "minecraft_java_server" => Ok(Self::Server),
            other => Err(EngineError::Config(format!(
                "unknown Modrinth domain '{other}'"
            ))),
        }
    }
}

/// Supported sort indices for Modrinth `/v3/search`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SortIndex {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl SortIndex {
    /// Return the Modrinth query parameter token for this sort order.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Downloads => "downloads",
            Self::Follows => "follows",
            Self::Newest => "newest",
            Self::Updated => "updated",
        }
    }

    /// Parse a sort index string (case-insensitive).
    pub fn from_name(name: &str) -> Result<Self, EngineError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "relevance" => Ok(Self::Relevance),
            "downloads" => Ok(Self::Downloads),
            "follows" => Ok(Self::Follows),
            "newest" => Ok(Self::Newest),
            "updated" => Ok(Self::Updated),
            other => Err(EngineError::Config(format!(
                "invalid search sort index '{other}'; expected one of: relevance, downloads, follows, newest, updated"
            ))),
        }
    }
}

/// Typed search parameters passed from Python into `oxirinth-engine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchFilters {
    pub query: String,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    pub categories: Vec<String>,
    pub environment: Option<String>,
    pub sort: SortIndex,
    pub limit: u32,
    pub offset: u32,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            query: String::new(),
            loaders: Vec::new(),
            game_versions: Vec::new(),
            categories: Vec::new(),
            environment: None,
            sort: SortIndex::Relevance,
            limit: 10,
            offset: 0,
        }
    }
}

/// Build Modrinth's 2D JSON `facets` parameter with automatic domain facet injection.
///
/// Outer array elements combine with `AND`; inner array elements combine with `OR`.
/// Values inside each filter group are normalized, sorted, and deduplicated so
/// equivalent filter sets produce identical cache keys.
pub fn build_facets_json(
    domain: DomainKind,
    filters: &SearchFilters,
) -> Result<String, EngineError> {
    let mut groups: Vec<Vec<String>> = vec![vec![domain.as_facet().to_owned()]];

    push_facet_group(&mut groups, domain.loader_facet_prefix(), &filters.loaders);
    push_facet_group(&mut groups, "game_versions", &filters.game_versions);
    push_facet_group(&mut groups, "categories", &filters.categories);

    if let Some(env) = filters
        .environment
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        groups.push(vec![format!("environment:{env}")]);
    }

    serde_json::to_string(&groups).map_err(|err| EngineError::Serialization(err.to_string()))
}

fn push_facet_group(groups: &mut Vec<Vec<String>>, prefix: &str, values: &[String]) {
    let mut entries: Vec<String> = values
        .iter()
        .map(|val| val.trim())
        .filter(|val| !val.is_empty())
        .map(|val| format!("{prefix}:{val}"))
        .collect();

    if entries.is_empty() {
        return;
    }

    entries.sort_unstable();
    entries.dedup();
    groups.push(entries);
}

#[derive(Deserialize)]
struct ProjectTypeProbe<'a> {
    #[serde(borrow)]
    project_types: Vec<&'a str>,
}

#[derive(Deserialize)]
struct ProjectIdentProbe<'a> {
    #[serde(borrow)]
    id: &'a str,
    #[serde(borrow)]
    slug: &'a str,
}

/// Zero-copy check confirming a `/v3/project` JSON payload belongs to `expected`.
pub fn verify_project_domain(
    raw_json: &[u8],
    expected: DomainKind,
    identifier: &str,
) -> Result<(), EngineError> {
    let probe: ProjectTypeProbe<'_> = serde_json::from_slice(raw_json).map_err(|err| {
        EngineError::Serialization(format!(
            "failed to inspect project_types for '{identifier}': {err}"
        ))
    })?;

    let expected_token = expected.as_project_type();
    if probe.project_types.contains(&expected_token) {
        return Ok(());
    }

    Err(EngineError::TypeMismatch {
        expected: expected_token,
        actual: probe.project_types.into_iter().map(str::to_owned).collect(),
        identifier: identifier.to_owned(),
    })
}

/// Extract `(id, slug)` from a Modrinth `/v3/project` JSON object without full model decoding.
pub fn extract_project_id_slug(raw_json: &[u8]) -> Result<(String, String), EngineError> {
    let probe: ProjectIdentProbe<'_> = serde_json::from_slice(raw_json).map_err(|err| {
        EngineError::Serialization(format!("failed to extract project id/slug: {err}"))
    })?;

    Ok((probe.id.to_owned(), probe.slug.to_owned()))
}

/// Split a JSON array payload into raw per-element UTF-8 JSON `Bytes` without re-serializing.
pub fn split_json_array_raw(raw_json: &[u8]) -> Result<Vec<Bytes>, EngineError> {
    let items: Vec<Box<RawValue>> = serde_json::from_slice(raw_json).map_err(|err| {
        EngineError::Serialization(format!("failed to split JSON array response: {err}"))
    })?;

    Ok(items
        .into_iter()
        .map(|item| Bytes::copy_from_slice(item.get().as_bytes()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_kind_maps_all_seven_domains() {
        assert_eq!(
            DomainKind::Server.as_project_type(),
            "minecraft_java_server"
        );
        assert_eq!(
            DomainKind::Server.as_facet(),
            "project_types:minecraft_java_server"
        );
        assert_eq!(DomainKind::Modpack.loader_facet_prefix(), "mrpack_loaders");
        assert_eq!(DomainKind::Server.loader_facet_prefix(), "mrpack_loaders");
        assert_eq!(DomainKind::Plugin.loader_facet_prefix(), "loaders");

        for domain in DomainKind::ALL {
            let parsed = DomainKind::from_name(domain.as_project_type()).unwrap();
            assert_eq!(parsed, domain);
        }
        assert_eq!(DomainKind::from_name("server").unwrap(), DomainKind::Server);
    }

    #[test]
    fn builds_deterministic_v3_facets_json() {
        let filters = SearchFilters {
            query: "permissions".to_owned(),
            loaders: vec!["purpur".to_owned(), "paper".to_owned(), "paper".to_owned()],
            game_versions: vec!["1.21.1".to_owned(), "1.20.4".to_owned()],
            categories: vec!["utility".to_owned()],
            environment: Some("server_only".to_owned()),
            ..SearchFilters::default()
        };

        let facets = build_facets_json(DomainKind::Plugin, &filters).unwrap();
        assert_eq!(
            facets,
            r#"[["project_types:plugin"],["loaders:paper","loaders:purpur"],["game_versions:1.20.4","game_versions:1.21.1"],["categories:utility"],["environment:server_only"]]"#
        );

        let server_facets = build_facets_json(
            DomainKind::Server,
            &SearchFilters {
                loaders: vec!["neoforge".to_owned()],
                ..SearchFilters::default()
            },
        )
        .unwrap();
        assert_eq!(
            server_facets,
            r#"[["project_types:minecraft_java_server"],["mrpack_loaders:neoforge"]]"#
        );
    }

    #[test]
    fn verifies_multi_type_projects_and_rejects_mismatches() {
        let luckperms = br#"{"id":"Vebnzrzj","slug":"luckperms","project_types":["mod","plugin"]}"#;
        assert!(verify_project_domain(luckperms, DomainKind::Plugin, "luckperms").is_ok());
        assert!(verify_project_domain(luckperms, DomainKind::Mod, "luckperms").is_ok());

        let sodium = br#"{"id":"AANobbMI","slug":"sodium","project_types":["mod"]}"#;
        let err = verify_project_domain(sodium, DomainKind::Plugin, "sodium").unwrap_err();
        match err {
            EngineError::TypeMismatch {
                expected,
                actual,
                identifier,
            } => {
                assert_eq!(expected, "plugin");
                assert_eq!(actual, vec!["mod".to_owned()]);
                assert_eq!(identifier, "sodium");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let (id, slug) = extract_project_id_slug(sodium).unwrap();
        assert_eq!(id, "AANobbMI");
        assert_eq!(slug, "sodium");
    }

    #[test]
    fn splits_json_array_preserving_raw_objects() {
        let payload = br#"[{"id":"a","n":1},{"id":"b","n":2}]"#;
        let items = split_json_array_raw(payload).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_ref(), br#"{"id":"a","n":1}"#);
        assert_eq!(items[1].as_ref(), br#"{"id":"b","n":2}"#);
    }
}
