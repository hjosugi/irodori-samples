use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::catalog::{Engine, SeedMode};
use crate::process::{ProcessRunner, RunOptions, SharedRunner};
use crate::runtime::{ContainerRuntime, RuntimeOps};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckSummary {
    pub project_files: usize,
    pub engines: usize,
    pub compose_files: usize,
}

pub fn run_repository_checks(
    root: &Path,
    catalog: &[Engine],
    runner: SharedRunner,
) -> Result<CheckSummary> {
    let runtime = ContainerRuntime::detect(root.to_path_buf(), runner.clone())?;
    let files = repository_files(root, runner.as_ref())?;
    let mut failures = Vec::new();

    for relative in &files {
        let absolute = root.join(relative);
        if relative.ends_with(".json") {
            match fs::read_to_string(&absolute)
                .with_context(|| format!("could not read {relative}"))
                .and_then(|source| {
                    serde_json::from_str::<serde_json::Value>(&source)
                        .with_context(|| format!("{relative}: invalid JSON"))
                }) {
                Ok(_) => {}
                Err(error) => failures.push(error.to_string()),
            }
        }
        if is_compose_file(relative)
            && let Err(error) = runtime.validate_compose(&absolute)
        {
            failures.push(format!("{relative}: Compose validation failed: {error}"));
        }
    }
    if !failures.is_empty() {
        bail!("sample checks failed:\n{}", failures.join("\n"));
    }

    let compose_files = check_connections(root, catalog)?;
    Ok(CheckSummary {
        project_files: files.len(),
        engines: catalog.len(),
        compose_files,
    })
}

fn repository_files(root: &Path, runner: &dyn ProcessRunner) -> Result<Vec<String>> {
    let output = runner.run(
        "git",
        &strings(&["ls-files", "--cached", "--others", "--exclude-standard"]),
        RunOptions::cwd(root),
    )?;
    Ok(output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn is_compose_file(path: &str) -> bool {
    let Some(name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    (name == "compose.yaml" || name == "compose.yml")
        || (name.starts_with("compose.") && (name.ends_with(".yaml") || name.ends_with(".yml")))
}

fn check_connections(root: &Path, catalog: &[Engine]) -> Result<usize> {
    let path = root.join("CONNECTIONS.md");
    let document =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let mut rows = BTreeMap::<String, Vec<String>>::new();
    let mut documented_ports = BTreeMap::<String, String>::new();
    for line in document.lines().filter(|line| line.starts_with("| ")) {
        let cells = line
            .split('|')
            .skip(1)
            .take_while(|cell| !cell.is_empty())
            .map(|cell| cell.trim().to_owned())
            .collect::<Vec<_>>();
        if cells.len() == 6
            && cells[0]
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            if cells[1].chars().all(|character| character.is_ascii_digit()) {
                documented_ports.insert(cells[0].clone(), cells[1].clone());
            }
            rows.insert(cells[0].clone(), cells);
        }
    }

    let mut problems = Vec::new();
    for engine in catalog {
        let Some(row) = rows.get(&engine.id) else {
            problems.push(format!(
                "{}: missing from the engine table in CONNECTIONS.md",
                engine.id
            ));
            continue;
        };
        let expected_seed = match engine.seed {
            SeedMode::Init => "init hook",
            SeedMode::Manual => "`task seed`",
            SeedMode::None => "—",
        };
        if row[5] != expected_seed {
            problems.push(format!(
                "{}: catalog seed mode '{}' disagrees with '{}' in CONNECTIONS.md",
                engine.id, engine.seed, row[5]
            ));
        }
        if engine.connection.is_empty() {
            problems.push(format!(
                "{}: missing from the 'URLs to paste' block",
                engine.id
            ));
        }
    }

    let mut compose_count = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let compose = entry.path().join("compose.yaml");
        if !compose.is_file() {
            continue;
        }
        let source = fs::read_to_string(&compose)?;
        let Some(published) = first_published_port(&source) else {
            continue;
        };
        compose_count += 1;
        let engine = entry.file_name().to_string_lossy().into_owned();
        match documented_ports.get(&engine) {
            None => problems.push(format!(
                "{engine}: has a compose.yaml but no row in CONNECTIONS.md"
            )),
            Some(expected) if expected != &published => problems.push(format!(
                "{engine}: CONNECTIONS.md says {expected}, compose publishes {published}"
            )),
            Some(_) => {}
        }
    }

    if !problems.is_empty() {
        bail!(
            "CONNECTIONS.md is out of date:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(compose_count)
}

fn first_published_port(source: &str) -> Option<String> {
    for line in source.lines() {
        let mut rest = line;
        while let Some(start) = rest.find('"') {
            rest = &rest[start + 1..];
            let Some(end) = rest.find('"') else {
                break;
            };
            let quoted = &rest[..end];
            if let Some((host, container)) = quoted.split_once(':')
                && !host.is_empty()
                && !container.is_empty()
                && host.chars().all(|character| character.is_ascii_digit())
                && container
                    .chars()
                    .all(|character| character.is_ascii_digit())
            {
                return Some(host.to_owned());
            }
            rest = &rest[end + 1..];
        }
    }
    None
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_file_detection_includes_variants() {
        assert!(is_compose_file("postgres/compose.yaml"));
        assert!(is_compose_file("postgres/compose.tls.yaml"));
        assert!(!is_compose_file("Taskfile.yml"));
    }

    #[test]
    fn published_port_parser_ignores_unquoted_values() {
        assert_eq!(
            first_published_port("ports:\n  - \"55432:5432\"\n"),
            Some("55432".to_owned())
        );
        assert_eq!(first_published_port("ports:\n  - 55432:5432\n"), None);
    }
}
