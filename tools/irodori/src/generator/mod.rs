mod data;
mod document;
mod relational;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use data::{Counts, Dataset};

pub const DEFAULT_SCALE: f64 = 0.02;
pub const DEFAULT_SEED: u32 = 20_260_807;
const OUTPUT_PATHS: &[&str] = &[
    "postgres/01_samples.sql",
    "mysql/01_samples.sql",
    "mongodb/01_samples.js",
    "oracle/01_samples.sql",
    "sqlserver/01_samples.sql",
    "clickhouse/01_samples.sql",
    "sqlite/01_samples.sql",
    "duckdb/01_samples.sql",
    "cassandra/01_samples.cql",
    "neo4j/01_samples.cypher",
    "memgraph/01_samples.cypher",
    "redis/01_samples.redis",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorConfig {
    pub scale: f64,
    pub seed: u32,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            scale: DEFAULT_SCALE,
            seed: DEFAULT_SEED,
        }
    }
}

impl GeneratorConfig {
    pub fn new(scale: f64, seed: u32) -> Result<Self> {
        if !scale.is_finite() || scale <= 0.0 {
            bail!("scale must be a finite number greater than zero");
        }
        if scale * 50_000.0 > u32::MAX as f64 {
            bail!("scale is too large");
        }
        Ok(Self { scale, seed })
    }

    pub fn from_environment() -> Result<Self> {
        Self::with_overrides(None, None)
    }

    pub fn with_overrides(scale: Option<f64>, seed: Option<u32>) -> Result<Self> {
        let scale = match scale {
            Some(scale) => scale,
            None => optional_environment("SCALE")?
                .map(|value| parse_scale(&value))
                .transpose()?
                .unwrap_or(DEFAULT_SCALE),
        };
        let seed = match seed {
            Some(seed) => seed,
            None => optional_environment("SEED")?
                .map(|value| parse_seed(&value))
                .transpose()?
                .unwrap_or(DEFAULT_SEED),
        };
        Self::new(scale, seed)
    }
}

pub fn parse_scale(value: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .with_context(|| format!("invalid scale '{value}'"))
}

pub fn parse_seed(value: &str) -> Result<u32> {
    value
        .parse::<u32>()
        .with_context(|| format!("invalid seed '{value}' (expected 0..={})", u32::MAX))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFile {
    pub relative_path: &'static str,
    pub body: String,
}

impl GeneratedFile {
    pub(super) fn new(relative_path: &'static str, mut body: String) -> Self {
        if !body.ends_with('\n') {
            body.push('\n');
        }
        Self {
            relative_path,
            body,
        }
    }

    pub fn line_count(&self) -> usize {
        self.body.trim().lines().count()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationSummary {
    pub files: usize,
    pub customers: usize,
    pub products: usize,
    pub orders: usize,
    pub order_items: usize,
    pub events: usize,
}

pub fn render_files(repository_root: &Path, config: GeneratorConfig) -> Result<Vec<GeneratedFile>> {
    Ok(render_bundle(repository_root, config)?.0)
}

fn render_bundle(
    repository_root: &Path,
    config: GeneratorConfig,
) -> Result<(Vec<GeneratedFile>, GenerationSummary)> {
    GeneratorConfig::new(config.scale, config.seed)?;
    let counts = Counts::from_scale(config.scale);
    let dataset = data::build(counts, config.seed);
    let mut files = relational::emit(repository_root, &dataset, counts, config)?;
    files.extend(document::emit(&dataset, config));
    files.sort_by_key(|file| {
        OUTPUT_PATHS
            .iter()
            .position(|path| *path == file.relative_path)
            .expect("every emitter has a declared output path")
    });
    let summary = GenerationSummary {
        files: files.len(),
        customers: dataset.customers.len(),
        products: dataset.products.len(),
        orders: dataset.orders.len(),
        order_items: dataset.order_items.len(),
        events: dataset.events.len(),
    };
    Ok((files, summary))
}

pub fn generate(
    repository_root: &Path,
    output_root: &Path,
    config: GeneratorConfig,
    mut report: impl FnMut(&str),
) -> Result<GenerationSummary> {
    let (files, summary) = render_bundle(repository_root, config)?;
    report(&format!(
        "seed={} scale={}",
        config.seed,
        number(config.scale)
    ));
    for (index, file) in files.iter().enumerate() {
        write_file(output_root, file, index)?;
        report(&format!(
            "  {:<34} {:>6} lines",
            file.relative_path,
            file.line_count()
        ));
    }

    report("");
    report(&format!(
        "  {} customers, {} products, {} orders, {} order lines, {} events",
        summary.customers, summary.products, summary.orders, summary.order_items, summary.events
    ));
    report(&format!(
        "  {} rows total",
        summary.customers
            + summary.products
            + summary.orders
            + summary.order_items
            + summary.events
    ));

    Ok(summary)
}

fn optional_environment(name: &str) -> Result<Option<String>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => bail!("{name} is not valid Unicode"),
    }
}

fn write_file(output_root: &Path, file: &GeneratedFile, index: usize) -> Result<()> {
    let destination = output_root.join(file.relative_path);
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("{} has no parent directory", destination.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("cannot create output directory {}", parent.display()))?;
    let name = destination
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "output filename is not valid Unicode: {}",
                destination.display()
            )
        })?;
    let temporary = parent.join(format!(
        ".{name}.irodori-{}-{index}.tmp",
        std::process::id()
    ));
    fs::write(&temporary, file.body.as_bytes())
        .with_context(|| format!("cannot stage {}", destination.display()))?;
    replace_file(&temporary, &destination)
        .with_context(|| format!("cannot replace generated fixture {}", destination.display()))
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<()> {
    match fs::rename(temporary, destination) {
        Ok(()) => return Ok(()),
        Err(error) if !destination.exists() => return Err(error.into()),
        Err(_) => {}
    }

    // Windows does not replace an existing destination with rename. Move the
    // old file aside and restore it if the second rename fails.
    let name = destination
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("fixture");
    let backup =
        destination.with_file_name(format!(".{name}.irodori-{}.backup", std::process::id()));
    fs::rename(destination, &backup)?;
    if let Err(error) = fs::rename(temporary, destination) {
        let _ = fs::rename(&backup, destination);
        return Err(error.into());
    }
    fs::remove_file(backup)?;
    Ok(())
}

fn banner(engine: &str, dataset: &Dataset, counts: Counts, config: GeneratorConfig) -> String {
    format!(
        "-- Irodori Table sample data — {engine}.\n\
         -- Generated by task generate (seed={}, scale={}).\n\
         -- Do not edit by hand; re-run the generator instead.\n\
         -- {} customers, {} products, {} orders, {} order lines, {} events.\n",
        config.seed,
        number(config.scale),
        counts.customers,
        counts.products,
        dataset.orders.len(),
        dataset.order_items.len(),
        dataset.events.len()
    )
}

fn inserts<T>(
    table: &str,
    columns: &[&str],
    rows: &[T],
    batch_size: usize,
    values: impl Fn(&T) -> Vec<String>,
) -> String {
    rows.chunks(batch_size)
        .map(|chunk| {
            let tuples = chunk
                .iter()
                .map(|row| format!("  ({})", values(row).join(", ")))
                .collect::<Vec<_>>()
                .join(",\n");
            format!(
                "INSERT INTO {table} ({}) VALUES\n{tuples};",
                columns.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn one_insert<T>(
    table: &str,
    columns: &[&str],
    rows: &[T],
    values: impl Fn(&T) -> Vec<String>,
) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "INSERT INTO {table} ({}) VALUES ({});",
                columns.join(", "),
                values(row).join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sql_quote(value: Option<&str>) -> String {
    value.map_or_else(
        || "NULL".to_owned(),
        |value| format!("'{}'", value.replace('\'', "''")),
    )
}

fn national_quote(value: Option<&str>) -> String {
    value.map_or_else(
        || "NULL".to_owned(),
        |value| format!("N'{}'", value.replace('\'', "''")),
    )
}

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("sample values are JSON serializable")
}

fn json_sql<T: Serialize>(value: &T) -> String {
    sql_quote(Some(&json(value)))
}

fn metadata_json(value: &data::Metadata) -> String {
    format!(
        "{{\"segment\":{},\"churn_risk\":{},\"newsletter\":{},\"locale\":{}}}",
        json(&value.segment),
        number(value.churn_risk),
        value.newsletter,
        json(&value.locale)
    )
}

fn payload_json(value: &data::Payload) -> String {
    format!(
        "{{\"path\":{},\"ab_bucket\":{},\"value\":{}}}",
        json(&value.path),
        json(&value.ab_bucket),
        number(value.value)
    )
}

fn number(value: f64) -> String {
    value.to_string()
}

fn sql_bool(value: bool) -> String {
    if value { "TRUE" } else { "FALSE" }.to_owned()
}

fn integer_bool(value: bool) -> String {
    if value { "1" } else { "0" }.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_root;

    #[test]
    fn default_render_is_byte_identical_to_committed_fixtures() {
        let root = repository_root();
        let generated_files = render_files(&root, GeneratorConfig::default()).unwrap();
        assert_eq!(
            generated_files
                .iter()
                .map(|file| file.relative_path)
                .collect::<Vec<_>>(),
            OUTPUT_PATHS
        );
        for generated in generated_files {
            let expected = fs::read_to_string(root.join(generated.relative_path)).unwrap();
            if generated.body != expected {
                let line = generated
                    .body
                    .lines()
                    .zip(expected.lines())
                    .position(|(actual, expected)| actual != expected)
                    .map_or_else(
                        || generated.body.lines().count().min(expected.lines().count()) + 1,
                        |index| index + 1,
                    );
                let actual = generated.body.lines().nth(line - 1).unwrap_or("<EOF>");
                let expected = expected.lines().nth(line - 1).unwrap_or("<EOF>");
                panic!(
                    "{} differs at line {line}\nexpected: {expected}\n  actual: {actual}",
                    generated.relative_path
                );
            }
        }
    }

    #[test]
    fn invalid_scale_is_rejected() {
        assert!(GeneratorConfig::new(0.0, DEFAULT_SEED).is_err());
        assert!(GeneratorConfig::new(f64::NAN, DEFAULT_SEED).is_err());
    }

    #[test]
    fn custom_scale_and_seed_are_deterministic() {
        let root = repository_root();
        let config = GeneratorConfig::new(0.001, 42).unwrap();
        let first = render_files(&root, config).unwrap();
        let second = render_files(&root, config).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 12);
        assert!(
            first[0]
                .body
                .contains("-- 10 customers, 2 products, 30 orders, ")
        );
        assert!(first[0].body.contains(", 50 events.\n"));
        assert!(first[0].body.contains("(seed=42, scale=0.001)"));
    }
}
