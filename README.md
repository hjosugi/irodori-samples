# irodori-samples

Local database containers and sample queries for Irodori Table.

## Use

```sh
make db-up DB=postgres
make db-verify DB=postgres
make db-down DB=postgres
```

The scripts use Podman when available, otherwise Docker.

Common `DB` values:

- `postgres`
- `mysql`
- `mariadb`
- `timescaledb`
- `cockroachdb`
- `yugabytedb`
- `tidb`
- `sqlserver`
- `mongodb`
- `oracle`

SQLite and DuckDB are embedded and do not need containers.

## Files

- `<engine>/compose.yaml`: one database container.
- `<engine>/01_samples.sql`: seed data when available.
- `projects/<engine>/queries.*`: queries to try in Irodori Table.
- `db-feature-samples.json`: machine-readable sample catalog.

License: `MIT OR 0BSD`.

## License

0BSD. You can use, copy, modify, and distribute this project for almost any purpose.
