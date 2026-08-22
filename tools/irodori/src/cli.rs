use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::catalog::{Engine, load_catalog};
use crate::certificates::issue_certificates;
use crate::checks::run_repository_checks;
use crate::discover_root;
use crate::generator::{GeneratorConfig, generate, parse_scale, parse_seed};
use crate::manager::{DoctorCheck, SampleManager};
use crate::process::{SharedRunner, SystemRunner};
use crate::runtime::Status;
use crate::tui::run_tui;

#[derive(Clone, Debug, PartialEq)]
pub struct Arguments {
    pub command: String,
    pub positionals: Vec<String>,
    pub variant: String,
    pub timeout: Duration,
    pub tail: u32,
    pub scale: Option<f64>,
    pub seed: Option<u32>,
    pub output: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub json: bool,
}

pub fn run_cli(values: impl IntoIterator<Item = String>) -> Result<i32> {
    let arguments = parse_arguments(values)?;
    if arguments.command == "help" {
        println!("{USAGE}");
        return Ok(0);
    }
    if arguments.command == "version" {
        println!("irodori {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }

    let root = discover_root(arguments.root.as_deref())?;
    let catalog = load_catalog(&root)?;
    let runner: SharedRunner = Arc::new(SystemRunner);
    let quiet = arguments.command == "tui";
    let mut manager = SampleManager::native(
        root.clone(),
        catalog.clone(),
        Arc::clone(&runner),
        move |message| {
            if !quiet {
                println!("{message}");
            }
        },
    );
    let engine = arguments
        .positionals
        .first()
        .cloned()
        .or_else(|| env::var("IRODORI_ENGINE").ok());

    match arguments.command.as_str() {
        "list" => print_catalog(&catalog, arguments.json)?,
        "tui" => run_tui(&mut manager)?,
        "up" => {
            manager.up(required_engine(engine.as_deref())?, &arguments.variant)?;
        }
        "start" => {
            manager.start(
                required_engine(engine.as_deref())?,
                &arguments.variant,
                arguments.timeout,
            )?;
        }
        "stop" => {
            manager.stop(required_engine(engine.as_deref())?, &arguments.variant)?;
        }
        "down" => {
            manager.down(
                required_engine(engine.as_deref())?,
                &arguments.variant,
                true,
            )?;
        }
        "reset" => {
            manager.reset(
                required_engine(engine.as_deref())?,
                &arguments.variant,
                arguments.timeout,
            )?;
        }
        "seed" => {
            manager.seed(required_engine(engine.as_deref())?)?;
        }
        "status" => {
            let statuses = manager.statuses()?;
            print_statuses(&catalog, &statuses, arguments.json)?;
        }
        "logs" => println!(
            "{}",
            manager.logs(
                required_engine(engine.as_deref())?,
                &arguments.variant,
                arguments.tail,
            )?
        ),
        "stop-all" => {
            manager.stop_all(false)?;
        }
        "down-all" => {
            manager.stop_all(true)?;
        }
        "urls" => print_connections(&manager, arguments.json)?,
        "doctor" => {
            let checks = manager.doctor();
            print_doctor(&checks, arguments.json)?;
            if checks.iter().any(|check| check.required && !check.ok) {
                return Ok(1);
            }
        }
        "certs" => println!(
            "{}",
            issue_certificates(&root, arguments.output.as_deref(), runner.as_ref())?
        ),
        "check" => {
            let summary = run_repository_checks(&root, &catalog, runner)?;
            println!(
                "sample checks passed ({} project files)",
                summary.project_files
            );
            println!(
                "connection reference matches {} engines and {} compose files",
                summary.engines, summary.compose_files
            );
        }
        "generate" => {
            let config = GeneratorConfig::with_overrides(arguments.scale, arguments.seed)?;
            generate(
                &root,
                arguments.output.as_deref().unwrap_or(&root),
                config,
                |message| println!("{message}"),
            )?;
        }
        command => bail!("unknown command '{command}'\n\n{USAGE}"),
    }
    Ok(0)
}

pub fn parse_arguments(values: impl IntoIterator<Item = String>) -> Result<Arguments> {
    let mut values = values.into_iter();
    let mut command = values.next().unwrap_or_else(|| "help".to_owned());
    if matches!(command.as_str(), "-h" | "--help") {
        command = "help".to_owned();
    } else if command == "--version" {
        command = "version".to_owned();
    }
    let mut arguments = Arguments {
        command,
        positionals: Vec::new(),
        variant: "default".to_owned(),
        timeout: Duration::from_secs(120),
        tail: 80,
        scale: None,
        seed: None,
        output: None,
        root: None,
        json: false,
    };
    let mut remaining = values.peekable();
    while let Some(value) = remaining.next() {
        match value.as_str() {
            "--json" => arguments.json = true,
            "--variant" => arguments.variant = required_option(&value, remaining.next())?,
            "--timeout" => {
                arguments.timeout = Duration::from_secs(positive_integer(
                    &required_option(&value, remaining.next())?,
                    &value,
                )? as u64)
            }
            "--tail" => {
                arguments.tail =
                    positive_integer(&required_option(&value, remaining.next())?, &value)?
            }
            "--scale" => {
                arguments.scale = Some(parse_scale(&required_option(&value, remaining.next())?)?)
            }
            "--seed" => {
                arguments.seed = Some(parse_seed(&required_option(&value, remaining.next())?)?)
            }
            "--output" => {
                arguments.output = Some(PathBuf::from(required_option(&value, remaining.next())?))
            }
            "--root" => {
                arguments.root = Some(PathBuf::from(required_option(&value, remaining.next())?))
            }
            option if option.starts_with("--") => bail!("unknown option '{option}'"),
            positional => arguments.positionals.push(positional.to_owned()),
        }
    }
    Ok(arguments)
}

fn required_engine(value: Option<&str>) -> Result<&str> {
    value.ok_or_else(|| anyhow::anyhow!("an engine is required (for example: postgres)"))
}

fn required_option(option: &str, value: Option<String>) -> Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{option} needs a value"))
}

fn positive_integer(value: &str, option: &str) -> Result<u32> {
    match value.parse::<u32>() {
        Ok(number) if number > 0 => Ok(number),
        _ => bail!("{option} must be a positive integer"),
    }
}

#[derive(Serialize)]
struct EngineOutput<'a> {
    id: &'a str,
    family: &'a str,
    seed: crate::catalog::SeedMode,
    embedded: bool,
    connection: &'a str,
}

fn print_catalog(catalog: &[Engine], json: bool) -> Result<()> {
    if json {
        let output = catalog
            .iter()
            .map(|engine| EngineOutput {
                id: &engine.id,
                family: &engine.family,
                seed: engine.seed,
                embedded: engine.embedded,
                connection: &engine.connection,
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    println!("Engines (use 'task tui' for interactive management):");
    for engine in catalog {
        println!(
            "  {:<15} {:<16} seed: {}",
            engine.id,
            engine.family,
            engine.seed.label()
        );
    }
    Ok(())
}

fn print_statuses(
    catalog: &[Engine],
    statuses: &BTreeMap<String, Status>,
    json: bool,
) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(statuses)?);
        return Ok(());
    }
    println!("{:<16} {:<10} DETAIL", "ENGINE", "STATE");
    for engine in catalog {
        let fallback = Status {
            state: "unknown".to_owned(),
            detail: String::new(),
            ports: String::new(),
        };
        let status = statuses.get(&engine.id).unwrap_or(&fallback);
        println!("{:<16} {:<10} {}", engine.id, status.state, status.detail);
    }
    Ok(())
}

fn print_connections(manager: &SampleManager, json: bool) -> Result<()> {
    let connections = manager.connections();
    if json {
        let output = connections.iter().copied().collect::<BTreeMap<_, _>>();
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    for (id, connection) in connections {
        println!("{id:<14} {connection}");
    }
    Ok(())
}

fn print_doctor(checks: &[DoctorCheck], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(checks)?);
        return Ok(());
    }
    for check in checks {
        let marker = if check.ok {
            "ok"
        } else if check.required {
            "FAIL"
        } else {
            "optional"
        };
        println!("{marker:<8} {:<18} {}", check.name, check.detail);
    }
    Ok(())
}

const USAGE: &str = "irodori-samples native manager

Usage:
  task tui                         interactive launcher
  task start -- postgres          start, wait, and seed when needed
  task stop -- postgres           stop and preserve data
  task reset -- postgres          delete data and recreate a ready engine
  task down -- postgres           delete containers and data
  task status                     show every engine
  task logs -- postgres           show recent logs

Low-level and utility commands:
  task up -- postgres             compose up without waiting or manual seeding
  task seed -- redis              apply a generated seed
  task stop:all                   stop all engines and preserve data
  task down:all                   delete all sample containers and data
  task urls                       print connection strings
  task doctor                     check local prerequisites
  task list                       list supported engines
  task generate                   regenerate every committed seed fixture

Direct binary options:
  --variant tls|host  --timeout SECONDS  --tail LINES  --json  --root PATH
  generate: --scale NUMBER  --seed INTEGER  --output DIRECTORY";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_parsed_without_a_platform_shell() {
        assert_eq!(
            parse_arguments([
                "start".into(),
                "postgres".into(),
                "--variant".into(),
                "tls".into(),
                "--timeout".into(),
                "45".into(),
            ])
            .unwrap(),
            Arguments {
                command: "start".into(),
                positionals: vec!["postgres".into()],
                variant: "tls".into(),
                timeout: Duration::from_secs(45),
                tail: 80,
                scale: None,
                seed: None,
                output: None,
                root: None,
                json: false,
            }
        );
    }

    #[test]
    fn invalid_numeric_options_are_rejected() {
        assert!(parse_arguments(["logs".into(), "--tail".into(), "0".into()]).is_err());
    }

    #[test]
    fn native_generator_options_are_parsed() {
        let arguments = parse_arguments([
            "generate".into(),
            "--scale".into(),
            "0.1".into(),
            "--seed".into(),
            "42".into(),
            "--output".into(),
            "fixtures".into(),
        ])
        .unwrap();
        assert_eq!(arguments.command, "generate");
        assert_eq!(arguments.scale, Some(0.1));
        assert_eq!(arguments.seed, Some(42));
        assert_eq!(arguments.output, Some(PathBuf::from("fixtures")));
    }
}
