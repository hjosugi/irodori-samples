<!-- i18n: language-switcher -->
[English](CONNECTIONS.md) | [日本語](CONNECTIONS.ja.md)

# Connection reference

Everything Irodori Table needs, per engine. `task urls` prints the same thing.

**The defaults are uniform on purpose** — where an engine lets us choose, it is
`irodori` / `irodori` with a database named `samples`. Only the engines that
refuse those values differ, and each one says why below. Nothing here is a
secret; these containers listen on localhost and hold generated data.

## 30-second start

```sh
task start -- postgres       # starts it and waits until it is ready
# paste this into Irodori Table:
#   postgres://irodori:irodori@localhost:55432/samples
```

`task start` handles both seed paths: seven engines load their data from an init
hook, while the manager seeds the other supported engines after they become
ready. You can also select and manage them interactively:

```sh
task tui
```

## The defaults

| | Value |
|---|---|
| Host | `localhost` (published on the loopback interface only) |
| User | `irodori` |
| Password | `irodori` |
| Database | `samples` |

## Every engine

Ports are in the `5xxxx` block. The database playground in the irodori-table
workspace uses `56xxx`, so both can run at the same time.

| Engine | Port | User | Password | Database | Seeded by |
|---|--:|---|---|---|---|
| postgres | 55432 | irodori | irodori | samples | init hook |
| mysql | 55306 | irodori | irodori | samples | init hook |
| mariadb | 55307 | irodori | irodori | samples | init hook |
| mongodb | 57017 | irodori | irodori | samples | init hook |
| oracle | 55521 | irodori | irodori | `FREEPDB1` | init hook |
| clickhouse | 58123 | irodori | irodori | samples | init hook |
| timescaledb | 55433 | irodori | irodori | samples | init hook |
| cockroachdb | 55257 | `root` | *(none)* | `defaultdb` | `task seed` |
| yugabytedb | 55434 | `yugabyte` | *(none)* | `yugabyte` | `task seed` |
| tidb | 54000 | `root` | *(none)* | `test` | `task seed` |
| sqlserver | 51433 | `sa` | `Irodori_Strong!23` | samples | `task seed` |
| redis | 56379 | — | irodori | `0` | `task seed` |
| neo4j | 57687 | `neo4j` | `irodoripass` | `neo4j` | `task seed` |
| memgraph | 57688 | — | *(none)* | `memgraph` | `task seed` |
| cassandra | 59042 | — | *(none)* | keyspace samples | `task seed` |
| scylladb | 59043 | — | *(none)* | keyspace samples | `task seed` |
| questdb | 58812 | `admin` | `quest` | `qdb` | — |
| influxdb | 58086 | — | *(none)* | samples | — |
| elasticsearch | 59200 | — | *(none)* | indices | — |
| openSearch | 59201 | — | *(none)* | indices | — |
| qdrant | 56333 | — | *(none)* | collections | — |
| arangodb | 58529 | `root` | irodori | samples | — |
| dynamodb | 58000 | `irodori` | `irodori` | tables | — |
| sqlite | — | — | — | `sqlite/samples.db` | `task seed` |
| duckdb | — | — | — | `duckdb/samples.duckdb` | `task seed` |

### Why five engines break the pattern

Each of these rejects the shared defaults outright rather than warning:

- **SQL Server** will not start unless the `sa` password is 8+ characters with
  three of upper/lower/digit/symbol — hence `Irodori_Strong!23`. It also needs
  `TrustServerCertificate=true`, because the certificate is self-signed.
- **Neo4j** refuses any password under 8 characters, so it is `irodoripass`.
- **CockroachDB, YugabyteDB and TiDB** run in their insecure single-node modes,
  which have no password at all. CockroachDB additionally requires
  `sslmode=disable` to be explicit.
- **QuestDB** has no user provisioning in the open-source build; `admin` /
  `quest` are fixed.
- **Oracle** names its pluggable database `FREEPDB1`; that is a service name,
  not a schema, and the user `irodori` owns the tables inside it.

## Secure variants

Every container above is plaintext and password-only, which is the right default
for "does the query path work" and the wrong shape for testing authentication.
`compose.tls.yaml` alongside a `compose.yaml` starts a hardened second copy on a
different port, so `task db-verify` keeps seeing the insecure one unchanged.

```
task certs             # issue a local CA into tls/certs (gitignored)
task start:secure -- postgres
task down:secure  -- postgres
```

The certificates are generated on demand and trusted only by these containers.
Nothing here is a secret worth protecting — but certificates committed to a
repository teach the wrong habit and expire into confusion, so they are not.

### PostgreSQL (port 55433)

TLS is *required*: `pg_hba.conf` has `hostssl` rules and no `host` rule at all,
so a plaintext connection is refused rather than quietly accepted. That refusal
is what makes the other cases meaningful — against a server that also accepts
plaintext, every SSL mode "succeeds" whether or not TLS was negotiated.

| what | connection |
|---|---|
| scram over TLS | `postgres://irodori:irodori@localhost:55433/samples?sslmode=verify-full&sslrootcert=$PWD/tls/certs/ca.crt` |
| client certificate | `postgres://irodori_cert@localhost:55433/samples?sslmode=verify-full&sslrootcert=$PWD/tls/certs/ca.crt&sslcert=$PWD/tls/certs/client.crt&sslkey=$PWD/tls/certs/client.key` |
| plaintext | refused |

The client certificate's CN is `irodori_cert` and must match the database role
exactly — a mismatch reports an unknown *user*, not a bad certificate, which
sends you looking in the wrong place.

`client.pk8.key` is the same key in PKCS#8 form. `native-tls`, which some
connectors use, accepts only that form on Windows and macOS while the OpenSSL
backend takes either — so a PKCS#1 key works on Linux and fails on a colleague's
machine.

### Verifying a client against it

irodori-table's `tests/integration_db.rs` connects to this container through the
connection *form* rather than a hand-written URL:

```
IRODORI_TLS_CERTS=/path/to/irodori-samples/tls/certs \
  cargo test --test integration_db -- tls
```

That is what caught sqlx being compiled without TLS support at all
(irodori-table/irodori-table#229).

## URLs to paste

```
postgres      postgres://irodori:irodori@localhost:55432/samples
timescaledb   postgres://irodori:irodori@localhost:55433/samples
cockroachdb   postgres://root@localhost:55257/defaultdb?sslmode=disable
yugabytedb    postgres://yugabyte@localhost:55434/yugabyte?sslmode=disable
questdb       postgres://admin:quest@localhost:58812/qdb
mysql         mysql://irodori:irodori@localhost:55306/samples
mariadb       mysql://irodori:irodori@localhost:55307/samples
tidb          mysql://root@localhost:54000/test
sqlserver     server=tcp:localhost,51433;User Id=sa;Password=Irodori_Strong!23;TrustServerCertificate=true
oracle        localhost:55521/FREEPDB1                    irodori / irodori
mongodb       mongodb://irodori:irodori@localhost:57017/samples?authSource=admin
redis         redis://:irodori@localhost:56379/0
neo4j         bolt://localhost:57687                      neo4j / irodoripass
memgraph      bolt://localhost:57688                      (no auth)
cassandra     cql localhost:59042                         keyspace samples
scylladb      cql localhost:59043                         keyspace samples
clickhouse    http://irodori:irodori@localhost:58123/     database samples
influxdb      http://localhost:58086                      database samples
elasticsearch http://localhost:59200
openSearch    http://localhost:59201
qdrant        http://localhost:56333
arangodb      http://localhost:58529                      root / irodori
dynamodb      http://localhost:58000                      us-east-1, irodori / irodori
sqlite        sqlite/samples.db
duckdb        duckdb/samples.duckdb
```

## Which need a connector extension

Built into the desktop app: postgres, mysql, mariadb, sqlite, timescaledb,
cockroachdb, yugabytedb, tidb, questdb, clickhouse, influxdb, snowflake.

Shipped as `legacy-connectors` or as marketplace extensions — install from
Settings → Extensions if the app says the data source is unavailable:
sqlserver, oracle, mongodb, redis, neo4j, cassandra, scylladb, memgraph,
duckdb, elasticsearch, openSearch, qdrant, arangodb, dynamodb.

## Web consoles

| | URL |
|---|---|
| CockroachDB DB Console | http://localhost:55180 |
| Neo4j Browser | http://localhost:57474 |
| QuestDB Web Console | http://localhost:59000 |
| ArangoDB | http://localhost:58529 |
| YugabyteDB UI | http://localhost:55435 |
