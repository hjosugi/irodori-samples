use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::catalog::{ComposeTarget, Engine};
use crate::process::{
    ProcessOutput, ProcessRunner, RunOptions, SharedRunner, command_exists, compact_output,
    first_line,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Status {
    pub state: String,
    pub detail: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ports: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectTarget {
    project: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Container {
    name: String,
    status: String,
    ports: String,
    state: String,
    target: Option<ProjectTarget>,
}

pub trait RuntimeOps: Send {
    fn binary(&self) -> &str;
    fn compose(&self, target: &ComposeTarget, args: &[String]) -> Result<ProcessOutput>;
    fn exec(&self, container: &str, args: &[String], input: Option<&[u8]>)
    -> Result<ProcessOutput>;
    fn run_container(&self, args: &[String], input: Option<&[u8]>) -> Result<ProcessOutput>;
    fn version(&self) -> Result<(String, String)>;
    fn check_network(&self) -> Result<String>;
    fn statuses(&self, catalog: &[Engine]) -> Result<BTreeMap<String, Status>>;
    fn status(&self, target: &ComposeTarget, catalog: &[Engine]) -> Result<Status>;
    fn find_container(&self, target: &ComposeTarget, catalog: &[Engine]) -> Result<Option<String>>;
    fn validate_compose(&self, path: &Path) -> Result<()>;
}

pub struct ContainerRuntime {
    binary: String,
    root: PathBuf,
    runner: SharedRunner,
}

impl ContainerRuntime {
    pub fn new(binary: impl Into<String>, root: PathBuf, runner: SharedRunner) -> Self {
        Self {
            binary: binary.into(),
            root,
            runner,
        }
    }

    pub fn detect(root: PathBuf, runner: SharedRunner) -> Result<Self> {
        let requested = env::var("IRODORI_CONTAINER_RUNTIME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                env::var("ENGINE_BIN")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            });
        Self::detect_with(root, runner, requested.as_deref())
    }

    pub fn detect_with(
        root: PathBuf,
        runner: SharedRunner,
        requested: Option<&str>,
    ) -> Result<Self> {
        if let Some(binary) = requested {
            if let Err(reason) = runtime_availability(binary, runner.as_ref(), &root) {
                bail!("container runtime '{binary}' is unavailable: {reason}");
            }
            return Ok(Self::new(binary, root, runner));
        }

        let mut failures = Vec::new();
        for candidate in ["podman", "docker"] {
            match runtime_availability(candidate, runner.as_ref(), &root) {
                Ok(()) => return Ok(Self::new(candidate, root, runner)),
                Err(reason) => failures.push(format!("{candidate}: {reason}")),
            }
        }
        bail!(
            "an operational Podman or Docker installation is required ({})",
            failures.join("; ")
        )
    }

    fn containers(&self, catalog: &[Engine]) -> Result<Vec<Container>> {
        let output = self.runner.run(
            &self.binary,
            &strings(&[
                "ps",
                "-a",
                "--filter",
                "name=irodori-",
                "--format",
                "{{.Names}}\t{{.Status}}\t{{.Ports}}",
            ]),
            RunOptions::cwd(&self.root),
        )?;
        Ok(parse_containers(&output.stdout, catalog))
    }
}

impl RuntimeOps for ContainerRuntime {
    fn binary(&self) -> &str {
        &self.binary
    }

    fn compose(&self, target: &ComposeTarget, args: &[String]) -> Result<ProcessOutput> {
        let mut command_args = vec![
            "compose".to_owned(),
            "-f".to_owned(),
            target.compose_path.to_string_lossy().into_owned(),
        ];
        command_args.extend_from_slice(args);
        self.runner
            .run(&self.binary, &command_args, RunOptions::cwd(&self.root))
    }

    fn exec(
        &self,
        container: &str,
        args: &[String],
        input: Option<&[u8]>,
    ) -> Result<ProcessOutput> {
        let mut command_args = strings(&["exec", "-i", container]);
        command_args.extend_from_slice(args);
        let mut options = RunOptions::cwd(&self.root);
        if let Some(input) = input {
            options.input = Some(input.to_vec());
        }
        self.runner.run(&self.binary, &command_args, options)
    }

    fn run_container(&self, args: &[String], input: Option<&[u8]>) -> Result<ProcessOutput> {
        let mut command_args = strings(&["run", "--rm"]);
        command_args.extend_from_slice(args);
        let mut options = RunOptions::cwd(&self.root);
        if let Some(input) = input {
            options.input = Some(input.to_vec());
        }
        self.runner.run(&self.binary, &command_args, options)
    }

    fn version(&self) -> Result<(String, String)> {
        let runtime = self.runner.run(
            &self.binary,
            &["--version".to_owned()],
            RunOptions::cwd(&self.root),
        )?;
        let compose = self.runner.run(
            &self.binary,
            &strings(&["compose", "version"]),
            RunOptions::cwd(&self.root),
        )?;
        Ok((
            compact_output(if runtime.stdout.is_empty() {
                &runtime.stderr
            } else {
                &runtime.stdout
            }),
            compact_output(if compose.stdout.is_empty() {
                &compose.stderr
            } else {
                &compose.stdout
            }),
        ))
    }

    fn check_network(&self) -> Result<String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let name = format!("irodori-doctor-{}-{stamp}", std::process::id());
        let image = "docker.io/library/redis:7-alpine";
        self.runner.run(
            &self.binary,
            &strings(&["network", "create", "--driver", "bridge", &name]),
            RunOptions::cwd(&self.root),
        )?;

        let operation = (|| {
            let inspected = self.runner.run(
                &self.binary,
                &strings(&["image", "inspect", image]),
                RunOptions::cwd(&self.root).allow_failure(),
            )?;
            if inspected.code != 0 {
                return Ok(
                    "bridge network created; container probe skipped until an engine image is present"
                        .to_owned(),
                );
            }
            self.runner.run(
                &self.binary,
                &strings(&[
                    "run",
                    "--rm",
                    "--pull=never",
                    "--network",
                    &name,
                    "--entrypoint",
                    "/bin/true",
                    image,
                ]),
                RunOptions::cwd(&self.root),
            )?;
            Ok("bridge network container probe succeeded".to_owned())
        })();

        let cleanup = self.runner.run(
            &self.binary,
            &strings(&["network", "rm", &name]),
            RunOptions::cwd(&self.root),
        );
        match (operation, cleanup) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(message), Ok(_)) => Ok(message),
        }
    }

    fn statuses(&self, catalog: &[Engine]) -> Result<BTreeMap<String, Status>> {
        let containers = self.containers(catalog)?;
        let mut by_project: BTreeMap<String, &Container> = BTreeMap::new();
        for container in &containers {
            let Some(target) = &container.target else {
                continue;
            };
            let replace = by_project
                .get(&target.project)
                .is_none_or(|current| state_rank(&container.state) > state_rank(&current.state));
            if replace {
                by_project.insert(target.project.clone(), container);
            }
        }

        Ok(catalog
            .iter()
            .map(|engine| {
                let status = if engine.embedded {
                    embedded_status(engine)
                } else {
                    engine
                        .project
                        .as_ref()
                        .and_then(|project| by_project.get(project))
                        .map_or_else(absent_status, |container| status_from_container(container))
                };
                (engine.id.clone(), status)
            })
            .collect())
    }

    fn status(&self, target: &ComposeTarget, catalog: &[Engine]) -> Result<Status> {
        let mut matches = self
            .containers(catalog)?
            .into_iter()
            .filter(|container| {
                container
                    .target
                    .as_ref()
                    .is_some_and(|found| found.project == target.project)
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|container| std::cmp::Reverse(state_rank(&container.state)));
        Ok(matches
            .first()
            .map_or_else(absent_status, status_from_container))
    }

    fn find_container(&self, target: &ComposeTarget, catalog: &[Engine]) -> Result<Option<String>> {
        Ok(self
            .containers(catalog)?
            .into_iter()
            .find(|container| {
                container
                    .target
                    .as_ref()
                    .is_some_and(|found| found.project == target.project)
                    && matches!(container.state.as_str(), "healthy" | "running" | "starting")
            })
            .map(|container| container.name))
    }

    fn validate_compose(&self, path: &Path) -> Result<()> {
        self.runner.run(
            &self.binary,
            &[
                "compose".to_owned(),
                "-f".to_owned(),
                path.to_string_lossy().into_owned(),
                "config".to_owned(),
                "--quiet".to_owned(),
            ],
            RunOptions::cwd(&self.root),
        )?;
        Ok(())
    }
}

fn runtime_availability(binary: &str, runner: &dyn ProcessRunner, root: &Path) -> Result<()> {
    if !command_exists(runner, binary) {
        bail!("not found on PATH");
    }
    let info = runner.run(
        binary,
        &["info".to_owned()],
        RunOptions::cwd(root).allow_failure(),
    )?;
    if info.code != 0 {
        let detail = if info.stderr.is_empty() {
            &info.stdout
        } else {
            &info.stderr
        };
        bail!("{}", nonempty(first_line(detail), "engine is not running"));
    }
    let compose = runner.run(
        binary,
        &strings(&["compose", "version"]),
        RunOptions::cwd(root).allow_failure(),
    )?;
    if compose.code != 0 {
        let detail = if compose.stderr.is_empty() {
            &compose.stdout
        } else {
            &compose.stderr
        };
        bail!("{}", nonempty(first_line(detail), "Compose is unavailable"));
    }
    Ok(())
}

fn nonempty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

fn parse_containers(output: &str, catalog: &[Engine]) -> Vec<Container> {
    let targets = project_targets(catalog);
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_container_line)
        .map(|mut container| {
            container.target = match_project(&container.name, &targets);
            container
        })
        .collect()
}

fn parse_container_line(line: &str) -> Container {
    let mut fields = line.splitn(3, '\t');
    let name = fields.next().unwrap_or("").trim().to_owned();
    let status = fields.next().unwrap_or("").trim().to_owned();
    let ports = fields.next().unwrap_or("").trim().to_owned();
    Container {
        state: classify_status(&status).to_owned(),
        name,
        status,
        ports,
        target: None,
    }
}

pub fn classify_status(status: &str) -> &'static str {
    let value = status.to_ascii_lowercase();
    if value.contains("unhealthy") {
        "unhealthy"
    } else if value.contains("healthy") {
        "healthy"
    } else if ["starting", "created", "restarting", "configured"]
        .iter()
        .any(|word| value.contains(word))
    {
        "starting"
    } else if value.split_whitespace().any(|word| word == "up") || value.contains("running") {
        "running"
    } else if ["exited", "stopped", "dead"]
        .iter()
        .any(|word| value.contains(word))
    {
        "stopped"
    } else {
        "unknown"
    }
}

fn project_targets(catalog: &[Engine]) -> Vec<ProjectTarget> {
    let mut targets = catalog
        .iter()
        .flat_map(|engine| {
            engine
                .project
                .iter()
                .chain(engine.variants.values().map(|variant| &variant.project))
                .map(|project| ProjectTarget {
                    project: project.clone(),
                })
        })
        .collect::<Vec<_>>();
    targets.sort_by_key(|target| std::cmp::Reverse(target.project.len()));
    targets
}

fn match_project(name: &str, targets: &[ProjectTarget]) -> Option<ProjectTarget> {
    let normalized = name.to_ascii_lowercase();
    targets.iter().find_map(|target| {
        let prefix = target.project.to_ascii_lowercase();
        (normalized == prefix
            || normalized.starts_with(&format!("{prefix}-"))
            || normalized.starts_with(&format!("{prefix}_")))
        .then(|| target.clone())
    })
}

fn state_rank(state: &str) -> u8 {
    match state {
        "healthy" | "ready" => 6,
        "running" => 5,
        "starting" => 4,
        "unhealthy" => 3,
        "stopped" => 2,
        "unknown" => 1,
        _ => 0,
    }
}

fn status_from_container(container: &Container) -> Status {
    Status {
        state: container.state.clone(),
        detail: container.status.clone(),
        ports: container.ports.clone(),
    }
}

fn absent_status() -> Status {
    Status {
        state: "absent".to_owned(),
        detail: "not created".to_owned(),
        ports: String::new(),
    }
}

pub fn embedded_status(engine: &Engine) -> Status {
    let path = engine.data_path.as_ref().expect("embedded data path");
    if path.is_file() {
        Status {
            state: "ready".to_owned(),
            detail: path.display().to_string(),
            ports: String::new(),
        }
    } else {
        absent_status()
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    use super::*;
    use crate::catalog::load_catalog;
    use crate::repository_root;

    type FakeOperation = dyn Fn(&str, &[String]) -> Result<ProcessOutput> + Send + Sync;

    struct FakeRunner {
        calls: Arc<Mutex<Vec<Vec<String>>>>,
        operation: Arc<FakeOperation>,
    }

    impl ProcessRunner for FakeRunner {
        fn run(
            &self,
            command: &str,
            args: &[String],
            _options: RunOptions,
        ) -> Result<ProcessOutput> {
            self.calls.lock().unwrap().push(args.to_vec());
            (self.operation)(command, args)
        }
    }

    #[test]
    fn status_parser_accepts_docker_and_podman_wording() {
        assert_eq!(classify_status("Up 12 seconds (healthy)"), "healthy");
        assert_eq!(
            classify_status("Up 3 seconds (health: starting)"),
            "starting"
        );
        assert_eq!(classify_status("Running for 4 minutes"), "running");
        assert_eq!(classify_status("Exited (1) 2 seconds ago"), "stopped");
        let parsed = parse_container_line(
            "irodori-redis-redis-1\tUp 2 minutes (healthy)\t0.0.0.0:56379->6379/tcp",
        );
        assert_eq!(parsed.name, "irodori-redis-redis-1");
        assert_eq!(parsed.state, "healthy");
        assert_eq!(parsed.ports, "0.0.0.0:56379->6379/tcp");
    }

    #[test]
    fn tls_project_is_not_mistaken_for_default() {
        let catalog = load_catalog(&repository_root()).unwrap();
        let containers = parse_containers(
            "irodori-postgres-tls-postgres-tls-1\tUp 2 minutes (healthy)\t55433/tcp\n\
             irodori-redis_redis_1\tUp 1 minute (healthy)\t56379/tcp",
            &catalog,
        );
        assert_eq!(
            containers[0].target.as_ref().unwrap().project,
            "irodori-postgres-tls"
        );
        assert_eq!(
            containers[1].target.as_ref().unwrap().project,
            "irodori-redis"
        );
    }

    #[test]
    fn detection_falls_back_from_an_inoperable_runtime() {
        let runner: SharedRunner = Arc::new(FakeRunner {
            calls: Arc::new(Mutex::new(Vec::new())),
            operation: Arc::new(|command, args| {
                if command == "podman" && args.first().is_some_and(|value| value == "info") {
                    return Ok(ProcessOutput {
                        code: 125,
                        stderr: "Podman machine is stopped".into(),
                        ..ProcessOutput::default()
                    });
                }
                Ok(ProcessOutput {
                    stdout: "ok".into(),
                    ..ProcessOutput::default()
                })
            }),
        });
        let runtime = ContainerRuntime::detect_with(repository_root(), runner, None).unwrap();
        assert_eq!(runtime.binary(), "docker");
    }

    #[test]
    fn network_probe_always_removes_its_network() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner: SharedRunner = Arc::new(FakeRunner {
            calls: Arc::clone(&calls),
            operation: Arc::new(|_, args| {
                if args.first().is_some_and(|value| value == "run") {
                    bail!("bridge unavailable");
                }
                Ok(ProcessOutput::default())
            }),
        });
        let runtime = ContainerRuntime::new("podman", repository_root(), runner);
        assert!(
            runtime
                .check_network()
                .unwrap_err()
                .to_string()
                .contains("bridge unavailable")
        );
        let calls = calls.lock().unwrap();
        assert_eq!(calls.last().unwrap()[..2], ["network", "rm"]);
    }
}
