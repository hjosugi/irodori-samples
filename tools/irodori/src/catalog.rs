use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SeedMode {
    Init,
    Manual,
    None,
}

impl SeedMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Init => "init hook",
            Self::Manual => "managed",
            Self::None => "none",
        }
    }
}

impl fmt::Display for SeedMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Init => "init",
            Self::Manual => "manual",
            Self::None => "none",
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Variant {
    pub compose_path: PathBuf,
    pub project: String,
    pub has_healthcheck: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Engine {
    pub id: String,
    pub family: String,
    pub seed: SeedMode,
    pub embedded: bool,
    pub directory: PathBuf,
    pub compose_path: Option<PathBuf>,
    pub project: Option<String>,
    pub has_healthcheck: bool,
    pub variants: BTreeMap<String, Variant>,
    pub connection: String,
    pub data_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposeTarget {
    pub id: String,
    pub variant: String,
    pub compose_path: PathBuf,
    pub project: String,
    pub has_healthcheck: bool,
}

impl ComposeTarget {
    pub fn display_name(&self) -> String {
        if self.variant == "default" {
            self.id.clone()
        } else {
            format!("{} ({})", self.id, self.variant)
        }
    }
}

struct Definition {
    id: &'static str,
    family: &'static str,
    seed: SeedMode,
    embedded: bool,
}

const DEFINITIONS: &[Definition] = &[
    Definition {
        id: "postgres",
        family: "Relational",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "mysql",
        family: "Relational",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "mariadb",
        family: "Relational",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "sqlite",
        family: "Relational",
        seed: SeedMode::Manual,
        embedded: true,
    },
    Definition {
        id: "sqlserver",
        family: "Enterprise SQL",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "oracle",
        family: "Enterprise SQL",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "cockroachdb",
        family: "Distributed SQL",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "yugabytedb",
        family: "Distributed SQL",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "tidb",
        family: "Distributed SQL",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "timescaledb",
        family: "Time-series",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "questdb",
        family: "Time-series",
        seed: SeedMode::None,
        embedded: false,
    },
    Definition {
        id: "influxdb",
        family: "Time-series",
        seed: SeedMode::None,
        embedded: false,
    },
    Definition {
        id: "clickhouse",
        family: "Columnar",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "duckdb",
        family: "Columnar",
        seed: SeedMode::Manual,
        embedded: true,
    },
    Definition {
        id: "mongodb",
        family: "Document",
        seed: SeedMode::Init,
        embedded: false,
    },
    Definition {
        id: "redis",
        family: "Key-value",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "dynamodb",
        family: "Key-value",
        seed: SeedMode::None,
        embedded: false,
    },
    Definition {
        id: "neo4j",
        family: "Graph",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "memgraph",
        family: "Graph",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "arangodb",
        family: "Graph",
        seed: SeedMode::None,
        embedded: false,
    },
    Definition {
        id: "cassandra",
        family: "Wide-column",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "scylladb",
        family: "Wide-column",
        seed: SeedMode::Manual,
        embedded: false,
    },
    Definition {
        id: "elasticsearch",
        family: "Search",
        seed: SeedMode::None,
        embedded: false,
    },
    Definition {
        id: "openSearch",
        family: "Search",
        seed: SeedMode::None,
        embedded: false,
    },
    Definition {
        id: "qdrant",
        family: "Vector",
        seed: SeedMode::None,
        embedded: false,
    },
];

pub fn load_catalog(root: &Path) -> Result<Vec<Engine>> {
    let connections = read_connections(&root.join("CONNECTIONS.md"))?;
    DEFINITIONS
        .iter()
        .map(|definition| load_engine(root, definition, &connections))
        .collect()
}

fn load_engine(
    root: &Path,
    definition: &Definition,
    connections: &BTreeMap<String, String>,
) -> Result<Engine> {
    let directory = root.join(definition.id);
    let compose_path = (!definition.embedded).then(|| directory.join("compose.yaml"));
    if let Some(path) = &compose_path
        && !path.is_file()
    {
        bail!(
            "catalog entry '{}' has no {}",
            definition.id,
            path.display()
        );
    }

    let metadata = compose_path
        .as_deref()
        .map(read_compose_metadata)
        .transpose()?;
    let mut variants = BTreeMap::new();
    if !definition.embedded {
        for name in ["tls", "host"] {
            let path = directory.join(format!("compose.{name}.yaml"));
            if path.is_file() {
                let (project, has_healthcheck) = read_compose_metadata(&path)?;
                variants.insert(
                    name.to_owned(),
                    Variant {
                        compose_path: path,
                        project,
                        has_healthcheck,
                    },
                );
            }
        }
    }

    Ok(Engine {
        id: definition.id.to_owned(),
        family: definition.family.to_owned(),
        seed: definition.seed,
        embedded: definition.embedded,
        directory: directory.clone(),
        compose_path,
        project: metadata.as_ref().map(|value| value.0.clone()),
        has_healthcheck: metadata.is_some_and(|value| value.1),
        variants,
        connection: connections.get(definition.id).cloned().unwrap_or_default(),
        data_path: definition.embedded.then(|| {
            directory.join(if definition.id == "sqlite" {
                "samples.db"
            } else {
                "samples.duckdb"
            })
        }),
    })
}

pub fn find_engine<'a>(catalog: &'a [Engine], value: &str) -> Result<&'a Engine> {
    catalog
        .iter()
        .find(|engine| engine.id.eq_ignore_ascii_case(value.trim()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "unknown engine '{}'. Choose one of: {}",
                value,
                catalog
                    .iter()
                    .map(|engine| engine.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

pub fn target_for(engine: &Engine, variant: &str) -> Result<ComposeTarget> {
    if engine.embedded {
        bail!("{} is embedded and has no Compose target", engine.id);
    }
    if variant == "default" {
        return Ok(ComposeTarget {
            id: engine.id.clone(),
            variant: variant.to_owned(),
            compose_path: engine.compose_path.clone().expect("container compose path"),
            project: engine.project.clone().expect("container project"),
            has_healthcheck: engine.has_healthcheck,
        });
    }
    let target = engine.variants.get(variant).ok_or_else(|| {
        let mut available = vec!["default".to_owned()];
        available.extend(engine.variants.keys().cloned());
        anyhow::anyhow!(
            "{} has no '{}' variant (available: {})",
            engine.id,
            variant,
            available.join(", ")
        )
    })?;
    Ok(ComposeTarget {
        id: engine.id.clone(),
        variant: variant.to_owned(),
        compose_path: target.compose_path.clone(),
        project: target.project.clone(),
        has_healthcheck: target.has_healthcheck,
    })
}

fn read_compose_metadata(path: &Path) -> Result<(String, bool)> {
    let source =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    let project = source
        .lines()
        .find_map(|line| {
            line.strip_prefix("name:")
                .map(|value| value.split('#').next().unwrap_or("").trim())
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            anyhow::anyhow!("{} must declare a top-level compose name", path.display())
        })?;
    let has_healthcheck = source.lines().any(|line| line.trim() == "healthcheck:");
    Ok((project, has_healthcheck))
}

fn read_connections(path: &Path) -> Result<BTreeMap<String, String>> {
    let source =
        fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))?;
    let section = source
        .split_once("## URLs to paste")
        .map(|(_, rest)| rest)
        .unwrap_or("")
        .split("\n## ")
        .next()
        .unwrap_or("");
    let block = section
        .split_once("```")
        .and_then(|(_, rest)| rest.split_once("```").map(|(body, _)| body))
        .unwrap_or("");
    let mut connections = BTreeMap::new();
    for definition in DEFINITIONS {
        if let Some(line) = block.lines().find(|line| {
            line.strip_prefix(definition.id)
                .is_some_and(|rest| rest.chars().next().is_some_and(char::is_whitespace))
        }) {
            connections.insert(
                definition.id.to_owned(),
                line[definition.id.len()..].trim().to_owned(),
            );
        }
    }
    Ok(connections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_root;

    #[test]
    fn catalog_describes_every_engine() {
        let catalog = load_catalog(&repository_root()).expect("catalog");
        assert_eq!(catalog.len(), 25);
        assert_eq!(catalog.iter().filter(|engine| engine.embedded).count(), 2);
        assert_eq!(
            find_engine(&catalog, "OPENSEARCH").unwrap().id,
            "openSearch"
        );
        assert!(
            find_engine(&catalog, "postgres")
                .unwrap()
                .connection
                .starts_with("postgres://")
        );
        assert_eq!(
            find_engine(&catalog, "timescaledb").unwrap().seed,
            SeedMode::Init
        );
    }

    #[test]
    fn compose_variants_keep_their_project_identity() {
        let catalog = load_catalog(&repository_root()).expect("catalog");
        let postgres = find_engine(&catalog, "postgres").unwrap();
        let tls = target_for(postgres, "tls").unwrap();
        assert_eq!(postgres.project.as_deref(), Some("irodori-postgres"));
        assert_eq!(tls.project, "irodori-postgres-tls");
        assert_eq!(tls.variant, "tls");
        assert!(tls.has_healthcheck);
    }
}
