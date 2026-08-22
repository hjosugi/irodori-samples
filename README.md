<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# irodori-samples

Run local database samples for Irodori Table from one cross-platform native
tool. The same Rust binary powers the [go-task](https://taskfile.dev) commands,
the interactive terminal UI, and the direct CLI.

## What you get

- One command to start an engine, wait until it is ready, and seed it.
- A TUI for browsing and managing all 25 supported engines.
- The same deterministic sample dataset across compatible databases.
- A native manager that builds and runs on Windows, macOS, and Linux.
- No Node.js requirement for lifecycle management or fixture generation.

## Quick start

### Requirements

| Requirement | When it is needed |
|---|---|
| go-task 3.x | Running the documented `task` commands |
| Rust 1.85+ with Cargo | The first build and rebuilds after source changes |
| Docker Desktop or Podman | Container-backed database engines |
| `sqlite3` or `duckdb` CLI | Only when using the matching embedded engine |
| OpenSSL | Only for `task certs` and secure variants |

After the first build, `task doctor` checks the container runtime, networking,
catalog, and optional engine-specific tools.

`doctor` validates the full repository setup, so it reports a missing
Docker/Podman runtime as a failure. SQLite and DuckDB themselves can still run
without a container runtime when their matching CLI is installed.

### First launch

From the repository root:

```sh
task doctor
task tui
```

The first command that needs the native tool builds an optimized binary at:

- Windows: `.build/irodori/bin/irodori.exe`
- macOS and Linux: `.build/irodori/bin/irodori`

Task reuses that binary until the Rust source changes. After it has been built,
running it does not require the Rust toolchain or Node.js.

In the TUI, select an engine with the arrow keys and press Enter. The manager
starts it, waits for readiness, and applies a seed when required.

### CLI alternative

Use the same manager without the TUI in scripts and CI:

```sh
task start -- postgres     # start, wait, and seed when needed
task status                # show every engine
task stop -- postgres      # stop and preserve data
```

`DB=postgres` is also accepted when passing arguments after `--` is
inconvenient:

```sh
task start DB=postgres
```

## Choose the right lifecycle command

| Command | Result | Data |
|---|---|---|
| `task start -- postgres` | Start, wait, and seed if needed | Preserved |
| `task up -- postgres` | Low-level Compose start; no readiness wait or manual seed | Preserved |
| `task stop -- postgres` | Stop the engine | Preserved |
| `task seed -- redis` | Apply or recreate generated sample contents | May be replaced |
| `task reset -- postgres` | Delete data, then recreate a ready engine | Deleted |
| `task down -- postgres` | Delete the engine resources and local data | Deleted |
| `task status` | Show container and embedded-engine status | Unchanged |
| `task logs -- postgres` | Show recent container logs | Unchanged |

> `seed`, `reset`, `down`, and `down:all` can overwrite or delete local sample
> data. The TUI asks for confirmation before reset and delete actions.

`task start` is the normal ready-to-use entry point. Use `task up` only when
you specifically want the lower-level Compose behavior.

## TUI controls

| Key | Action |
|---|---|
| `Enter` | Start, wait, and seed when supported |
| `s` | Stop and preserve data |
| `e` | Apply or recreate the seed |
| `r` | Reset and recreate after confirmation |
| `d` | Delete resources and data after confirmation |
| `l` | Show recent logs |
| `R` | Refresh status |
| `q` | Quit |

The TUI requires an interactive terminal and runs on Windows, macOS, and Linux.

## Container runtime and variants

The manager detects Docker and Podman. When both are operational, it prefers
Podman. Select one explicitly with a Task variable:

```sh
task doctor RUNTIME=docker
task start DB=postgres RUNTIME=podman
```

Useful variants:

```sh
task start:secure -- postgres  # local TLS certificates and secure Compose variant
task start:host -- postgres    # Linux host-network fallback
```

Run `task certs` to issue the local CA and certificates used by secure
containers. Individual database images may have their own CPU-architecture
availability constraints even though the manager itself is cross-platform.

## Supported engines

| Family | Engines |
|---|---|
| Relational | `postgres` `mysql` `mariadb` `sqlite` |
| Enterprise SQL | `sqlserver` `oracle` |
| Distributed SQL | `cockroachdb` `yugabytedb` `tidb` |
| Time-series | `timescaledb` `questdb` `influxdb` |
| Columnar | `clickhouse` `duckdb` |
| Document | `mongodb` |
| Key-value | `redis` `dynamodb` |
| Graph | `neo4j` `memgraph` `arangodb` |
| Wide-column | `cassandra` `scylladb` |
| Search | `elasticsearch` `openSearch` |
| Vector | `qdrant` |

SQLite and DuckDB are embedded and do not require a container. Their respective
CLI is required only when starting or seeding that engine.

Credentials, ports, and connection URLs are documented in
[CONNECTIONS.md](CONNECTIONS.md). Print the same URL list with:

```sh
task urls
```

## Sample data and seed generation

Seed-capable engines receive the same deterministic dataset by default:

| Entity | Rows |
|---|---:|
| Customers | 200 |
| Products | 40 |
| Orders | 600 |
| Order lines | 2,066 |
| Events | 1,000 |

The fixtures include Unicode and bidirectional text, nullable values, exact
decimal aggregates, JSON, relationships, and engine-specific types.

### How seeds are loaded

- PostgreSQL, TimescaleDB, MySQL, MariaDB, MongoDB, Oracle, and ClickHouse use
  an image initialization hook.
- CockroachDB, YugabyteDB, TiDB, SQL Server, Redis, Neo4j, Memgraph, Cassandra,
  ScyllaDB, SQLite, and DuckDB are seeded by the native manager after startup.
- Other engines currently provide connection and feature fixtures without the
  generated commerce dataset.

`task start` handles these differences automatically.

### Regenerate committed fixtures

```sh
task generate              # defaults: SCALE=0.02, SEED=20260807
task generate SCALE=0.1    # larger deterministic dataset
task generate SEED=42      # different deterministic dataset
```

Generation runs inside the native Rust binary and rewrites the committed
`<engine>/01_samples.*` files. The default output is byte-for-byte
reproducible and covered by unit tests.

## Native implementation and platforms

All entry points share one implementation:

| Entry point | Intended use |
|---|---|
| `task ...` | Short, cross-platform project commands |
| `task tui` | Interactive engine management |
| `.build/irodori/bin/irodori ...` | Direct automation after building |

Lifecycle rules, runtime detection, seeding, certificate generation,
repository checks, fixture generation, CLI output, and TUI state live in the
Rust crate under `tools/irodori/`. The Taskfile remains a thin launcher.

The project has no Node.js package manifest or Node.js command. The remaining
MongoDB `.js` files are input for the Mongo shell, not a Node.js runtime
dependency.

CI builds and tests the native tool on Linux, macOS, and Windows. It also runs
the generator on every platform and verifies that committed fixtures do not
change.

## Validation and development

```sh
task list        # list engines and seed modes
task build       # explicitly build the native tool
task test        # unit and fixture-reproducibility tests
task lint        # rustfmt and clippy
task check       # all tests plus sample, Compose, and connection validation
```

After building, display the direct CLI help with:

```sh
.build/irodori/bin/irodori help
```

Append `.exe` on Windows.

## Repository layout

- `<engine>/compose.yaml`: one local database service.
- `<engine>/01_samples.*`: a generated seed where supported.
- `projects/<engine>/queries.*`: queries to try in Irodori Table.
- `tools/irodori/`: the native Rust manager, TUI, seeder, and generator.
- `generator/`: static PostgreSQL demo data consumed by the native generator.
- `db-feature-samples.json`: the machine-readable sample catalog.

## License

0BSD. You can use, copy, modify, and distribute this project for almost any
purpose.
