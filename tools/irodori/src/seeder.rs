use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::catalog::{Engine, SeedMode, target_for};
use crate::process::{ProcessOutput, RunOptions, SharedRunner, command_exists};
use crate::runtime::RuntimeOps;

const SQLCMD_PATHS: &[&str] = &[
    "/opt/mssql-tools18/bin/sqlcmd",
    "/opt/mssql-tools/bin/sqlcmd",
];

pub trait SeederOps: Send {
    fn seed(&self, engine: &Engine, runtime: Option<&dyn RuntimeOps>) -> Result<String>;
    fn ready(&self, engine: &Engine, runtime: &dyn RuntimeOps) -> bool;
    fn is_seeded(&self, engine: &Engine, runtime: Option<&dyn RuntimeOps>) -> bool;
}

pub struct Seeder {
    root: PathBuf,
    catalog: Vec<Engine>,
    runner: SharedRunner,
}

impl Seeder {
    pub fn new(root: PathBuf, catalog: Vec<Engine>, runner: SharedRunner) -> Self {
        Self {
            root,
            catalog,
            runner,
        }
    }

    fn seed_embedded(&self, engine: &Engine) -> Result<String> {
        let command = if engine.id == "sqlite" {
            "sqlite3"
        } else {
            "duckdb"
        };
        if !command_exists(self.runner.as_ref(), command) {
            let extension = if engine.id == "sqlite" {
                "db"
            } else {
                "duckdb"
            };
            bail!(
                "{command} is required to create {}/samples.{extension}",
                engine.id
            );
        }

        let data_path = engine.data_path.as_ref().expect("embedded data path");
        let suffix = unique_suffix();
        let temporary = sibling(data_path, &format!("tmp-{suffix}"));
        let backup = sibling(data_path, &format!("backup-{suffix}"));
        remove_file_if_present(&temporary)?;
        remove_file_if_present(&backup)?;
        let mut backed_up = false;

        let operation = (|| {
            self.runner.run(
                command,
                &[temporary.to_string_lossy().into_owned()],
                RunOptions::cwd(&self.root)
                    .input(self.seed_source(&format!("{}/01_samples.sql", engine.id))?),
            )?;
            if !temporary.is_file() {
                bail!(
                    "{command} completed without creating {}",
                    temporary.display()
                );
            }
            if data_path.exists() {
                fs::rename(data_path, &backup).with_context(|| {
                    format!(
                        "could not back up existing database {}",
                        data_path.display()
                    )
                })?;
                backed_up = true;
            }
            fs::rename(&temporary, data_path)
                .with_context(|| format!("could not replace database {}", data_path.display()))?;
            if backed_up {
                remove_file_if_present(&backup)?;
            }
            Ok(())
        })();

        if operation.is_err() && backed_up && !data_path.exists() && backup.exists() {
            fs::rename(&backup, data_path).with_context(|| {
                format!(
                    "could not restore previous database {}",
                    data_path.display()
                )
            })?;
        }
        let cleanup = remove_file_if_present(&temporary);
        operation?;
        cleanup?;
        Ok(format!("{}: created {}", engine.id, data_path.display()))
    }

    fn seed_source(&self, relative: &str) -> Result<Vec<u8>> {
        let path = self.root.join(relative);
        fs::read(&path).with_context(|| format!("could not read seed {}", path.display()))
    }

    fn find_container(&self, engine: &Engine, runtime: &dyn RuntimeOps) -> Result<String> {
        let target = target_for(engine, "default")?;
        runtime
            .find_container(&target, &self.catalog)?
            .ok_or_else(|| anyhow::anyhow!("{} is not running; start it before seeding", engine.id))
    }

    fn run_mysql_client(
        &self,
        runtime: &dyn RuntimeOps,
        container: &str,
        input: Option<&[u8]>,
        query: Option<&str>,
    ) -> Result<ProcessOutput> {
        let mut args = strings(&[
            "-i",
            "--network",
            &format!("container:{container}"),
            "docker.io/library/mysql:8.4",
            "mysql",
            "--protocol=tcp",
            "-h127.0.0.1",
            "-P4000",
            "-uroot",
            "--batch",
            "--skip-column-names",
        ]);
        if let Some(query) = query {
            args.extend(strings(&["--execute", query]));
        }
        args.push("test".to_owned());
        runtime.run_container(&args, input)
    }

    fn run_sqlcmd(
        &self,
        runtime: &dyn RuntimeOps,
        container: &str,
        input: Option<&[u8]>,
        query: Option<&str>,
    ) -> Result<ProcessOutput> {
        let mut last_error = None;
        for command in SQLCMD_PATHS {
            let mut args = strings(&[
                command,
                "-S",
                "localhost",
                "-U",
                "sa",
                "-P",
                "Irodori_Strong!23",
                "-C",
                "-b",
            ]);
            if let Some(query) = query {
                args.extend(strings(&["-h", "-1", "-W", "-Q", query]));
            }
            match runtime.exec(container, &args, input) {
                Ok(output) => return Ok(output),
                Err(error) => {
                    let message = error.to_string().to_ascii_lowercase();
                    if !message.contains("not found") && !message.contains("no such file") {
                        return Err(error);
                    }
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("sqlcmd is unavailable")))
    }
}

impl SeederOps for Seeder {
    fn seed(&self, engine: &Engine, runtime: Option<&dyn RuntimeOps>) -> Result<String> {
        match engine.seed {
            SeedMode::Init => {
                return Ok(format!(
                    "{}: seed is loaded by the image init hook",
                    engine.id
                ));
            }
            SeedMode::None => {
                return Ok(format!(
                    "{}: no generated sample seed is available",
                    engine.id
                ));
            }
            SeedMode::Manual => {}
        }
        if engine.embedded {
            return self.seed_embedded(engine);
        }
        let runtime = runtime.ok_or_else(|| anyhow::anyhow!("container runtime is required"))?;
        let container = self.find_container(engine, runtime)?;

        match engine.id.as_str() {
            "cockroachdb" => {
                let seed = self.seed_source("postgres/01_samples.sql")?;
                runtime.exec(
                    &container,
                    &strings(&[
                        "cockroach",
                        "sql",
                        "--insecure",
                        "--database=defaultdb",
                        "--set=errexit=true",
                    ]),
                    Some(&seed),
                )?;
                Ok("cockroachdb: samples loaded into defaultdb".to_owned())
            }
            "yugabytedb" => {
                let seed = self.seed_source("postgres/01_samples.sql")?;
                runtime.exec(
                    &container,
                    &strings(&[
                        "bin/ysqlsh",
                        "-h",
                        "127.0.0.1",
                        "-U",
                        "yugabyte",
                        "-d",
                        "yugabyte",
                        "-v",
                        "ON_ERROR_STOP=1",
                    ]),
                    Some(&seed),
                )?;
                Ok("yugabytedb: samples loaded into yugabyte".to_owned())
            }
            "tidb" => {
                let seed = self.seed_source("mysql/01_samples.sql")?;
                self.run_mysql_client(runtime, &container, Some(&seed), None)?;
                Ok("tidb: samples loaded into test".to_owned())
            }
            "sqlserver" => {
                let seed = self.seed_source("sqlserver/01_samples.sql")?;
                self.run_sqlcmd(runtime, &container, Some(&seed), None)?;
                Ok("sqlserver: samples loaded into samples".to_owned())
            }
            "redis" => {
                let source = String::from_utf8(self.seed_source("redis/01_samples.redis")?)?;
                let source = source
                    .lines()
                    .filter(|line| !line.starts_with('#'))
                    .collect::<Vec<_>>()
                    .join("\n");
                runtime.exec(
                    &container,
                    &strings(&["redis-cli", "-a", "irodori", "--no-auth-warning"]),
                    Some(source.as_bytes()),
                )?;
                let count = runtime.exec(
                    &container,
                    &strings(&["redis-cli", "-a", "irodori", "--no-auth-warning", "DBSIZE"]),
                    None,
                )?;
                Ok(format!("redis: {} keys", count.stdout.trim()))
            }
            "neo4j" => {
                let seed = self.seed_source("neo4j/01_samples.cypher")?;
                runtime.exec(
                    &container,
                    &strings(&[
                        "cypher-shell",
                        "-u",
                        "neo4j",
                        "-p",
                        "irodoripass",
                        "--format",
                        "plain",
                    ]),
                    Some(&seed),
                )?;
                let count = runtime.exec(
                    &container,
                    &strings(&[
                        "cypher-shell",
                        "-u",
                        "neo4j",
                        "-p",
                        "irodoripass",
                        "--format",
                        "plain",
                        "match (n) return count(n)",
                    ]),
                    None,
                )?;
                Ok(format!(
                    "neo4j: {} nodes",
                    last_nonempty_line(&count.stdout)
                ))
            }
            "memgraph" => {
                let seed = self.seed_source("memgraph/01_samples.cypher")?;
                runtime.exec(&container, &strings(&["mgconsole"]), Some(&seed))?;
                let count = runtime.exec(
                    &container,
                    &strings(&["mgconsole", "--output-format=csv"]),
                    Some(b"MATCH (n) RETURN Count(n);\n"),
                )?;
                Ok(format!(
                    "memgraph: {} nodes",
                    last_nonempty_line(&count.stdout)
                ))
            }
            "cassandra" | "scylladb" => {
                let seed = self.seed_source("cassandra/01_samples.cql")?;
                runtime.exec(
                    &container,
                    &strings(&["cqlsh", "--request-timeout=120"]),
                    Some(&seed),
                )?;
                Ok(format!("{}: keyspace samples loaded", engine.id))
            }
            _ => bail!("no managed seed implementation for '{}'", engine.id),
        }
    }

    fn ready(&self, engine: &Engine, runtime: &dyn RuntimeOps) -> bool {
        if engine.embedded {
            return true;
        }
        if !matches!(
            engine.id.as_str(),
            "cockroachdb" | "yugabytedb" | "tidb" | "sqlserver" | "mongodb" | "oracle"
        ) {
            return true;
        }
        let Ok(container) = self.find_container(engine, runtime) else {
            return false;
        };
        let result = match engine.id.as_str() {
            "cockroachdb" => runtime.exec(
                &container,
                &strings(&[
                    "cockroach",
                    "sql",
                    "--insecure",
                    "--database=defaultdb",
                    "--execute=select 1",
                ]),
                None,
            ),
            "yugabytedb" => runtime.exec(
                &container,
                &strings(&[
                    "bin/ysqlsh",
                    "-h",
                    "127.0.0.1",
                    "-U",
                    "yugabyte",
                    "-d",
                    "yugabyte",
                    "-c",
                    "select 1",
                ]),
                None,
            ),
            "tidb" => self.run_mysql_client(runtime, &container, None, Some("select 1")),
            "sqlserver" => self.run_sqlcmd(runtime, &container, None, Some("select 1")),
            "mongodb" => runtime.exec(
                &container,
                &strings(&[
                    "mongosh",
                    "--quiet",
                    "--username",
                    "irodori",
                    "--password",
                    "irodori",
                    "--authenticationDatabase",
                    "admin",
                    "--eval",
                    "quit(db.adminCommand({ ping: 1 }).ok ? 0 : 2)",
                ]),
                None,
            ),
            "oracle" => runtime.exec(&container, &strings(&["/opt/oracle/healthcheck.sh"]), None),
            _ => return true,
        };
        result.is_ok()
    }

    fn is_seeded(&self, engine: &Engine, runtime: Option<&dyn RuntimeOps>) -> bool {
        if engine.seed != SeedMode::Manual {
            return true;
        }
        if engine.embedded {
            return engine.data_path.as_ref().is_some_and(|path| path.is_file());
        }
        let Some(runtime) = runtime else {
            return false;
        };
        let Ok(container) = self.find_container(engine, runtime) else {
            return false;
        };
        let result = match engine.id.as_str() {
            "cockroachdb" => runtime.exec(
                &container,
                &strings(&[
                    "cockroach",
                    "sql",
                    "--insecure",
                    "--database=defaultdb",
                    "--format=csv",
                    "--execute=SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'customers'",
                ]),
                None,
            ),
            "yugabytedb" => runtime.exec(
                &container,
                &strings(&[
                    "bin/ysqlsh",
                    "-h",
                    "127.0.0.1",
                    "-U",
                    "yugabyte",
                    "-d",
                    "yugabyte",
                    "-t",
                    "-A",
                    "-c",
                    "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'customers'",
                ]),
                None,
            ),
            "tidb" => self.run_mysql_client(
                runtime,
                &container,
                None,
                Some("SELECT count(*) FROM information_schema.tables WHERE table_schema = 'test' AND table_name = 'customers'"),
            ),
            "sqlserver" => self.run_sqlcmd(
                runtime,
                &container,
                None,
                Some("SET NOCOUNT ON; SELECT CASE WHEN OBJECT_ID('samples.dbo.customers') IS NULL THEN 0 ELSE 1 END"),
            ),
            "redis" => runtime.exec(
                &container,
                &strings(&[
                    "redis-cli",
                    "-a",
                    "irodori",
                    "--no-auth-warning",
                    "EXISTS",
                    "customer:1",
                ]),
                None,
            ),
            "neo4j" => runtime.exec(
                &container,
                &strings(&[
                    "cypher-shell",
                    "-u",
                    "neo4j",
                    "-p",
                    "irodoripass",
                    "--format",
                    "plain",
                    "match (n:Customer) return count(n)",
                ]),
                None,
            ),
            "memgraph" => runtime.exec(
                &container,
                &strings(&["mgconsole", "--output-format=csv"]),
                Some(b"MATCH (n:Customer) RETURN Count(n);\n"),
            ),
            "cassandra" | "scylladb" => runtime.exec(
                &container,
                &strings(&[
                    "cqlsh",
                    "--request-timeout=30",
                    "-e",
                    "SELECT table_name FROM system_schema.tables WHERE keyspace_name = 'samples' AND table_name = 'customers';",
                ]),
                None,
            ),
            _ => return false,
        };
        result.is_ok_and(|output| {
            if matches!(engine.id.as_str(), "cassandra" | "scylladb") {
                output
                    .stdout
                    .split_whitespace()
                    .any(|word| word == "customers")
            } else {
                has_positive_result(&output.stdout)
            }
        })
    }
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{name}.{suffix}"))
}

fn unique_suffix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{timestamp}", std::process::id())
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("could not remove {}", path.display())),
    }
}

fn last_nonempty_line(value: &str) -> &str {
    value
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .unwrap_or("?")
}

fn has_positive_result(value: &str) -> bool {
    value
        .lines()
        .any(|line| line.trim().parse::<u64>().is_ok_and(|number| number > 0))
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::Mutex;

    use super::*;
    use crate::catalog::{ComposeTarget, find_engine, load_catalog};
    use crate::process::ProcessRunner;
    use crate::repository_root;
    use crate::runtime::Status;

    type RuntimeCall = (Vec<String>, Option<Vec<u8>>);

    #[derive(Default)]
    struct FakeRuntime {
        calls: Mutex<Vec<RuntimeCall>>,
    }

    impl RuntimeOps for FakeRuntime {
        fn binary(&self) -> &str {
            "fake"
        }
        fn compose(&self, _: &ComposeTarget, _: &[String]) -> Result<ProcessOutput> {
            unreachable!()
        }
        fn exec(&self, _: &str, args: &[String], input: Option<&[u8]>) -> Result<ProcessOutput> {
            self.calls
                .lock()
                .unwrap()
                .push((args.to_vec(), input.map(<[u8]>::to_vec)));
            let stdout = if args.iter().any(|value| value == "EXISTS") {
                "1\n"
            } else if args.iter().any(|value| value == "DBSIZE") {
                "1234\n"
            } else {
                ""
            };
            Ok(ProcessOutput {
                stdout: stdout.into(),
                ..ProcessOutput::default()
            })
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
            unreachable!()
        }
        fn find_container(&self, _: &ComposeTarget, _: &[Engine]) -> Result<Option<String>> {
            Ok(Some("irodori-redis-redis-1".into()))
        }
        fn validate_compose(&self, _: &Path) -> Result<()> {
            unreachable!()
        }
    }

    struct NoopRunner;
    impl ProcessRunner for NoopRunner {
        fn run(&self, _: &str, _: &[String], _: RunOptions) -> Result<ProcessOutput> {
            Ok(ProcessOutput::default())
        }
    }

    #[test]
    fn redis_seed_and_detection_share_the_runtime() {
        let root = repository_root();
        let catalog = load_catalog(&root).unwrap();
        let redis = find_engine(&catalog, "redis").unwrap();
        let runtime = FakeRuntime::default();
        let seeder = Seeder::new(root, catalog.clone(), Arc::new(NoopRunner));
        assert!(seeder.is_seeded(redis, Some(&runtime)));
        assert_eq!(
            seeder.seed(redis, Some(&runtime)).unwrap(),
            "redis: 1234 keys"
        );
        let calls = runtime.calls.lock().unwrap();
        let seed = calls.iter().find_map(|(_, input)| input.as_ref()).unwrap();
        assert!(String::from_utf8_lossy(seed).contains("FLUSHALL"));
        assert!(
            !String::from_utf8_lossy(seed)
                .lines()
                .any(|line| line.starts_with('#'))
        );
    }

    #[test]
    fn embedded_failure_preserves_the_previous_database() {
        let root = repository_root();
        let catalog = load_catalog(&root).unwrap();
        let mut sqlite = find_engine(&catalog, "sqlite").unwrap().clone();
        let directory =
            std::env::temp_dir().join(format!("irodori-seeder-test-{}", unique_suffix()));
        fs::create_dir_all(&directory).unwrap();
        let data_path = directory.join("samples.db");
        fs::write(&data_path, b"existing database").unwrap();
        sqlite.data_path = Some(data_path.clone());
        let seeder = Seeder::new(root, catalog, Arc::new(NoopRunner));

        assert!(seeder.seed(&sqlite, None).is_err());
        assert_eq!(fs::read(&data_path).unwrap(), b"existing database");
        fs::remove_dir_all(directory).unwrap();
    }
}
