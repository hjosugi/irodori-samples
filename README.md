<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# irodori-samples

Local database containers and sample queries for Irodori Table.

## Use

Driven by [go-task](https://taskfile.dev). `task` on its own lists everything.

```sh
task up   -- postgres     # start it; most engines seed themselves on first boot
task seed -- redis        # only for engines whose image has no init hook
task down -- postgres     # stop it and drop the volume
task generate             # regenerate every <engine>/01_samples.* file
```

From the irodori-table checkout next door, `task db-verify DB=postgres` starts
one of these, runs the integration tests against it, and stops it again.

Podman is used when available, otherwise Docker.

Engines:

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

SQLite and DuckDB are embedded and need no container.

**All credentials and ports are in [CONNECTIONS.md](CONNECTIONS.md)** — `task urls` prints
the same list.

## The sample data

Every engine gets the same small dataset: 200 customers, 40 products,
600 orders, 2,066 order lines and 1,000 events — about 3,900 rows. It is
deliberately awkward in the ways a database client cares about: every 40th
customer has a non-ASCII name (Japanese, Korean, Arabic, emoji), `shipped_at`
and `note` are nullable and stay null on most rows, and `orders.subtotal` is the
exact sum of that order's lines so aggregates reconcile on any engine.

The seeds are generated, not hand-written: `node generator/generate.mjs`
rebuilds every `01_samples.*` from one deterministic dataset, so a query that
returns 42 against Postgres returns 42 against ClickHouse. `SCALE=0.1` makes it
five times bigger. For a full-size version — 196,393 rows across 28 engines —
see the database playground in the irodori-table workspace.

## Files

- `<engine>/compose.yaml`: one database container.
- `<engine>/01_samples.*`: the seed, in that engine's native format.
- `generator/`: the deterministic generator that produces those seeds.
- `scripts/seed.sh`: applies a seed to engines whose image has no init hook.
- `projects/<engine>/queries.*`: queries to try in Irodori Table.
- `db-feature-samples.json`: machine-readable sample catalog.

License: `MIT OR 0BSD`.

## License

0BSD. You can use, copy, modify, and distribute this project for almost any purpose.
