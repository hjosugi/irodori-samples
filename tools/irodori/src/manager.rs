use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::catalog::{ComposeTarget, Engine, SeedMode, find_engine, target_for};
use crate::process::{SharedRunner, command_exists};
use crate::runtime::{ContainerRuntime, RuntimeOps, Status};
use crate::seeder::{Seeder, SeederOps};

type RuntimeFactory = Box<dyn Fn() -> Result<Box<dyn RuntimeOps>> + Send>;
type Reporter = Box<dyn Fn(&str) + Send>;
type Delay = Box<dyn Fn(Duration) + Send>;

#[derive(Clone, Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub required: bool,
    pub detail: String,
}

pub struct SampleManager {
    catalog: Vec<Engine>,
    runtime: Option<Box<dyn RuntimeOps>>,
    runtime_factory: RuntimeFactory,
    seeder: Box<dyn SeederOps>,
    runner: SharedRunner,
    report: Reporter,
    delay: Delay,
}

impl SampleManager {
    pub fn native(
        root: PathBuf,
        catalog: Vec<Engine>,
        runner: SharedRunner,
        report: impl Fn(&str) + Send + 'static,
    ) -> Self {
        let factory_root = root.clone();
        let factory_runner = Arc::clone(&runner);
        let seeder = Seeder::new(root.clone(), catalog.clone(), Arc::clone(&runner));
        Self {
            catalog,
            runtime: None,
            runtime_factory: Box::new(move || {
                Ok(Box::new(ContainerRuntime::detect(
                    factory_root.clone(),
                    Arc::clone(&factory_runner),
                )?))
            }),
            seeder: Box::new(seeder),
            runner,
            report: Box::new(report),
            delay: Box::new(thread::sleep),
        }
    }

    #[cfg(test)]
    fn injected(
        catalog: Vec<Engine>,
        runtime_factory: RuntimeFactory,
        seeder: Box<dyn SeederOps>,
        runner: SharedRunner,
    ) -> Self {
        Self {
            catalog,
            runtime: None,
            runtime_factory,
            seeder,
            runner,
            report: Box::new(|_| {}),
            delay: Box::new(|_| {}),
        }
    }

    pub fn catalog(&self) -> &[Engine] {
        &self.catalog
    }

    pub fn engine(&self, value: &str) -> Result<&Engine> {
        find_engine(&self.catalog, value)
    }

    fn engine_owned(&self, value: &str) -> Result<Engine> {
        self.engine(value).cloned()
    }

    fn ensure_runtime(&mut self) -> Result<()> {
        if self.runtime.is_none() {
            self.runtime = Some((self.runtime_factory)()?);
        }
        Ok(())
    }

    fn runtime(&self) -> &dyn RuntimeOps {
        self.runtime.as_deref().expect("runtime initialized")
    }

    pub fn runtime_name(&mut self) -> Result<String> {
        self.ensure_runtime()?;
        Ok(self.runtime().binary().to_owned())
    }

    pub fn up(&mut self, value: &str, variant: &str) -> Result<String> {
        let engine = self.engine_owned(value)?;
        if engine.embedded {
            if !self.seeder.is_seeded(&engine, None) {
                self.seed(&engine.id)?;
            }
            let message = format!("{} is ready", engine.id);
            self.notify(&message);
            return Ok(message);
        }
        let target = target_for(&engine, variant)?;
        self.ensure_runtime()?;
        self.notify(&format!("starting {}...", target.display_name()));
        self.runtime().compose(&target, &strings(&["up", "-d"]))?;
        let message = format!("started {}", target.display_name());
        self.notify(&message);
        Ok(message)
    }

    pub fn start(&mut self, value: &str, variant: &str, timeout: Duration) -> Result<String> {
        let engine = self.engine_owned(value)?;
        if engine.embedded {
            if !self.seeder.is_seeded(&engine, None) {
                self.seed(&engine.id)?;
            }
            let message = with_connection(format!("{} is ready", engine.id), &engine.connection);
            self.notify(&message);
            return Ok(message);
        }
        let target = target_for(&engine, variant)?;
        self.up(&engine.id, variant)?;
        self.wait_until_ready(&engine, &target, timeout)?;

        if variant == "default" && engine.seed == SeedMode::Manual {
            let seeded = self.seeder.is_seeded(&engine, Some(self.runtime()));
            if !seeded {
                self.seed(&engine.id)?;
            }
        }
        let message = with_connection(
            format!("{} is ready", target.display_name()),
            &engine.connection,
        );
        self.notify(&message);
        Ok(message)
    }

    pub fn stop(&mut self, value: &str, variant: &str) -> Result<String> {
        let engine = self.engine_owned(value)?;
        if engine.embedded {
            let message = format!("{} is embedded; there is no process to stop", engine.id);
            self.notify(&message);
            return Ok(message);
        }
        let target = target_for(&engine, variant)?;
        self.ensure_runtime()?;
        self.notify(&format!("stopping {}...", target.display_name()));
        self.runtime().compose(&target, &strings(&["stop"]))?;
        let message = format!("stopped {}; data was preserved", target.display_name());
        self.notify(&message);
        Ok(message)
    }

    pub fn down(&mut self, value: &str, variant: &str, volumes: bool) -> Result<String> {
        let engine = self.engine_owned(value)?;
        if engine.embedded {
            if let Some(path) = &engine.data_path {
                remove_file_if_present(path)?;
            }
            let message = format!("removed {} sample database", engine.id);
            self.notify(&message);
            return Ok(message);
        }
        let target = target_for(&engine, variant)?;
        self.ensure_runtime()?;
        let data_suffix = if volumes { " and its data" } else { "" };
        self.notify(&format!(
            "removing {}{}...",
            target.display_name(),
            data_suffix
        ));
        let mut args = vec!["down".to_owned()];
        if volumes {
            args.push("-v".to_owned());
        }
        self.runtime().compose(&target, &args)?;
        let message = format!("removed {}{}", target.display_name(), data_suffix);
        self.notify(&message);
        Ok(message)
    }

    pub fn reset(&mut self, value: &str, variant: &str, timeout: Duration) -> Result<String> {
        let engine = self.engine_owned(value)?;
        self.notify(&format!("resetting {}...", engine.id));
        self.down(&engine.id, variant, true)?;
        self.start(&engine.id, variant, timeout)
    }

    pub fn seed(&mut self, value: &str) -> Result<String> {
        let engine = self.engine_owned(value)?;
        if engine.seed == SeedMode::Manual && !engine.embedded {
            self.ensure_runtime()?;
        }
        self.notify(&format!("seeding {}...", engine.id));
        let runtime = self.runtime.as_deref();
        let message = self.seeder.seed(&engine, runtime)?;
        self.notify(&message);
        Ok(message)
    }

    pub fn statuses(&mut self) -> Result<BTreeMap<String, Status>> {
        self.ensure_runtime()?;
        self.runtime().statuses(&self.catalog)
    }

    pub fn status(&mut self, value: &str, variant: &str) -> Result<Status> {
        let engine = self.engine_owned(value)?;
        if engine.embedded {
            return Ok(crate::runtime::embedded_status(&engine));
        }
        let target = target_for(&engine, variant)?;
        self.ensure_runtime()?;
        self.runtime().status(&target, &self.catalog)
    }

    pub fn stop_all(&mut self, destroy: bool) -> Result<String> {
        self.ensure_runtime()?;
        let mut failures = Vec::new();
        let targets = self
            .catalog
            .iter()
            .filter(|engine| !engine.embedded)
            .flat_map(|engine| {
                std::iter::once(target_for(engine, "default")).chain(
                    engine
                        .variants
                        .keys()
                        .map(|variant| target_for(engine, variant)),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        for target in targets {
            let args = if destroy {
                strings(&["down", "-v"])
            } else {
                strings(&["stop"])
            };
            if let Err(error) = self.runtime().compose(&target, &args) {
                failures.push(format!("{}: {error}", target.project));
            }
        }
        if destroy {
            for engine in self.catalog.iter().filter(|engine| engine.embedded) {
                if let Some(path) = &engine.data_path
                    && let Err(error) = remove_file_if_present(path)
                {
                    failures.push(format!("{}: {error}", engine.id));
                }
            }
        }
        if !failures.is_empty() {
            bail!(
                "some engines could not be {}:\n{}",
                if destroy { "removed" } else { "stopped" },
                failures.join("\n")
            );
        }
        let message = if destroy {
            "removed all sample engines and their data"
        } else {
            "stopped all container-backed sample engines; data was preserved"
        }
        .to_owned();
        self.notify(&message);
        Ok(message)
    }

    pub fn logs(&mut self, value: &str, variant: &str, tail: u32) -> Result<String> {
        let engine = self.engine_owned(value)?;
        if engine.embedded {
            return Ok(format!(
                "{} is embedded and has no container logs",
                engine.id
            ));
        }
        let target = target_for(&engine, variant)?;
        self.ensure_runtime()?;
        let output = self.runtime().compose(
            &target,
            &[
                "logs".to_owned(),
                "--no-color".to_owned(),
                "--tail".to_owned(),
                tail.to_string(),
            ],
        )?;
        Ok([output.stdout, output.stderr]
            .into_iter()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned())
    }

    pub fn connections(&self) -> Vec<(&str, &str)> {
        self.catalog
            .iter()
            .map(|engine| (engine.id.as_str(), engine.connection.as_str()))
            .collect()
    }

    pub fn doctor(&mut self) -> Vec<DoctorCheck> {
        let mut checks = vec![
            DoctorCheck {
                name: "native manager".to_owned(),
                ok: true,
                required: true,
                detail: format!(
                    "v{} {}/{}",
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ),
            },
            DoctorCheck {
                name: "catalog".to_owned(),
                ok: self.catalog.len() == 25,
                required: true,
                detail: format!("{} engines", self.catalog.len()),
            },
        ];

        match self.ensure_runtime() {
            Ok(()) => match self.runtime().version() {
                Ok((runtime, compose)) => {
                    checks.push(DoctorCheck {
                        name: "containers".to_owned(),
                        ok: true,
                        required: true,
                        detail: format!("{runtime}; {compose}"),
                    });
                    match self.runtime().check_network() {
                        Ok(detail) => checks.push(DoctorCheck {
                            name: "container network".to_owned(),
                            ok: true,
                            required: true,
                            detail,
                        }),
                        Err(error) => checks.push(DoctorCheck {
                            name: "container network".to_owned(),
                            ok: false,
                            required: true,
                            detail: error.to_string(),
                        }),
                    }
                }
                Err(error) => checks.push(DoctorCheck {
                    name: "containers".to_owned(),
                    ok: false,
                    required: true,
                    detail: error.to_string(),
                }),
            },
            Err(error) => checks.push(DoctorCheck {
                name: "containers".to_owned(),
                ok: false,
                required: true,
                detail: error.to_string(),
            }),
        }

        for (name, command) in [
            ("SQLite seed", "sqlite3"),
            ("DuckDB seed", "duckdb"),
            ("TLS certificates", "openssl"),
        ] {
            checks.push(DoctorCheck {
                name: name.to_owned(),
                ok: command_exists(self.runner.as_ref(), command),
                required: false,
                detail: command.to_owned(),
            });
        }
        checks
    }

    fn wait_until_ready(
        &mut self,
        engine: &Engine,
        target: &ComposeTarget,
        timeout: Duration,
    ) -> Result<()> {
        self.ensure_runtime()?;
        let deadline = Instant::now() + timeout;
        let mut last_state = "absent".to_owned();
        self.notify(&format!("waiting for {}...", target.display_name()));
        while Instant::now() < deadline {
            let status = self.runtime().status(target, &self.catalog)?;
            last_state = status.state.clone();
            if target.has_healthcheck && status.state == "healthy" {
                return Ok(());
            }
            if !target.has_healthcheck
                && status.state == "running"
                && self.seeder.ready(engine, self.runtime())
            {
                return Ok(());
            }
            if status.state == "stopped" {
                bail!(
                    "{} stopped before becoming ready ({})",
                    target.display_name(),
                    status.detail
                );
            }
            (self.delay)(Duration::from_millis(1_500));
        }
        bail!(
            "{} did not become ready within {}s (last state: {})",
            target.display_name(),
            timeout.as_secs(),
            last_state
        )
    }

    fn notify(&self, message: &str) {
        (self.report)(message);
    }
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn with_connection(mut message: String, connection: &str) -> String {
    if !connection.is_empty() {
        message.push('\n');
        message.push_str(connection);
    }
    message
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::catalog::Variant;
    use crate::process::{ProcessOutput, SystemRunner};

    struct MockRuntime {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl RuntimeOps for MockRuntime {
        fn binary(&self) -> &str {
            "fake"
        }
        fn compose(
            &self,
            _: &ComposeTarget,
            args: &[String],
        ) -> Result<crate::process::ProcessOutput> {
            self.events
                .lock()
                .unwrap()
                .push(format!("compose {}", args.join(" ")));
            Ok(ProcessOutput::default())
        }
        fn exec(&self, _: &str, _: &[String], _: Option<&[u8]>) -> Result<ProcessOutput> {
            unreachable!()
        }
        fn run_container(&self, _: &[String], _: Option<&[u8]>) -> Result<ProcessOutput> {
            unreachable!()
        }
        fn version(&self) -> Result<(String, String)> {
            unreachable!()
        }
        fn check_network(&self) -> Result<String> {
            unreachable!()
        }
        fn statuses(&self, _: &[Engine]) -> Result<BTreeMap<String, Status>> {
            unreachable!()
        }
        fn status(&self, _: &ComposeTarget, _: &[Engine]) -> Result<Status> {
            self.events.lock().unwrap().push("status".into());
            Ok(Status {
                state: "running".into(),
                detail: "running".into(),
                ports: String::new(),
            })
        }
        fn find_container(&self, _: &ComposeTarget, _: &[Engine]) -> Result<Option<String>> {
            unreachable!()
        }
        fn validate_compose(&self, _: &Path) -> Result<()> {
            unreachable!()
        }
    }

    struct MockSeeder {
        events: Arc<Mutex<Vec<String>>>,
        seeded: bool,
    }

    impl SeederOps for MockSeeder {
        fn seed(&self, engine: &Engine, _: Option<&dyn RuntimeOps>) -> Result<String> {
            self.events
                .lock()
                .unwrap()
                .push(format!("seed {}", engine.id));
            Ok(format!("{}: seeded", engine.id))
        }
        fn ready(&self, _: &Engine, _: &dyn RuntimeOps) -> bool {
            self.events.lock().unwrap().push("ready".into());
            true
        }
        fn is_seeded(&self, _: &Engine, _: Option<&dyn RuntimeOps>) -> bool {
            self.events.lock().unwrap().push("seeded".into());
            self.seeded
        }
    }

    fn fixture(seeded: bool) -> (SampleManager, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let engine = Engine {
            id: "demo".into(),
            family: "Test".into(),
            seed: SeedMode::Manual,
            embedded: false,
            directory: PathBuf::from("/repo/demo"),
            compose_path: Some(PathBuf::from("/repo/demo/compose.yaml")),
            project: Some("irodori-demo".into()),
            has_healthcheck: false,
            variants: BTreeMap::new(),
            connection: "demo://localhost".into(),
            data_path: None,
        };
        let runtime_events = Arc::clone(&events);
        let runtime_factory: RuntimeFactory = Box::new(move || {
            Ok(Box::new(MockRuntime {
                events: Arc::clone(&runtime_events),
            }))
        });
        let seeder = Box::new(MockSeeder {
            events: Arc::clone(&events),
            seeded,
        });
        let manager = SampleManager::injected(
            vec![engine],
            runtime_factory,
            seeder,
            Arc::new(SystemRunner),
        );
        (manager, events)
    }

    #[test]
    fn lifecycle_operations_share_one_manager() {
        let (mut manager, events) = fixture(false);
        assert!(
            manager
                .start("demo", "default", Duration::from_secs(1))
                .unwrap()
                .contains("ready")
        );
        assert_eq!(
            *events.lock().unwrap(),
            ["compose up -d", "status", "ready", "seeded", "seed demo"]
        );

        events.lock().unwrap().clear();
        manager.stop("demo", "default").unwrap();
        assert_eq!(*events.lock().unwrap(), ["compose stop"]);

        events.lock().unwrap().clear();
        manager
            .reset("demo", "default", Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            [
                "compose down -v",
                "compose up -d",
                "status",
                "ready",
                "seeded",
                "seed demo"
            ]
        );
    }

    #[test]
    fn start_preserves_an_existing_managed_dataset() {
        let (mut manager, events) = fixture(true);
        manager
            .start("demo", "default", Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            ["compose up -d", "status", "ready", "seeded"]
        );
    }

    #[test]
    fn all_engine_operations_include_variants() {
        let (mut manager, events) = fixture(false);
        manager.catalog[0].variants.insert(
            "tls".into(),
            Variant {
                compose_path: PathBuf::from("/repo/demo/compose.tls.yaml"),
                project: "irodori-demo-tls".into(),
                has_healthcheck: true,
            },
        );
        manager.stop_all(false).unwrap();
        assert_eq!(*events.lock().unwrap(), ["compose stop", "compose stop"]);
    }
}
