pub mod catalog;
pub mod certificates;
pub mod checks;
pub mod cli;
pub mod generator;
pub mod manager;
pub mod process;
pub mod runtime;
pub mod seeder;
pub mod tui;

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Finds the repository independently of the shell and current platform.
/// Task runs from the repository root, while `--root` and IRODORI_ROOT make a
/// downloaded binary usable from scripts and IDE launchers too.
pub fn discover_root(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = explicit {
        return validate_root(root);
    }
    if let Some(root) = env::var_os("IRODORI_ROOT") {
        return validate_root(Path::new(&root));
    }

    if let Ok(current) = env::current_dir()
        && let Some(root) = find_root_from(&current)
    {
        return Ok(root);
    }
    if let Ok(executable) = env::current_exe()
        && let Some(parent) = executable.parent()
        && let Some(root) = find_root_from(parent)
    {
        return Ok(root);
    }

    bail!("could not find the irodori-samples repository; run inside it or pass --root PATH")
}

fn find_root_from(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|candidate| is_root(candidate))
        .map(Path::to_path_buf)
}

fn validate_root(path: &Path) -> Result<PathBuf> {
    let root = path
        .canonicalize()
        .with_context(|| format!("cannot access repository root {}", path.display()))?;
    if !is_root(&root) {
        bail!(
            "{} is not an irodori-samples repository (Taskfile.yml and CONNECTIONS.md are required)",
            root.display()
        );
    }
    Ok(root)
}

fn is_root(path: &Path) -> bool {
    path.join("Taskfile.yml").is_file() && path.join("CONNECTIONS.md").is_file()
}

#[cfg(test)]
pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}
