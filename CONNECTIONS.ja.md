<!-- i18n: language-switcher -->
[English](CONNECTIONS.md) | [日本語](CONNECTIONS.ja.md)

# 接続情報

Irodori Table に入れる値の一覧。`task urls` でも同じものが出る。

**既定値は意図的に揃えてある** — エンジン側が選ばせてくれる限り
`irodori` / `irodori`、DB 名は `samples`。違うのは「その値を受け付けない」
エンジンだけで、理由は下に書いてある。秘密情報ではない（localhost 限定公開・
中身は生成データ）。

## 30秒で始める

```sh
task up -- postgres          # 起動と同時に初回投入まで終わる
# Irodori Table にこれを貼る:
#   postgres://irodori:irodori@localhost:55432/samples
```

`task up` だけで完結するのは6つ — PostgreSQL, MySQL, MariaDB, MongoDB,
Oracle, ClickHouse はイメージの init フックでシードを読む。
残りはもう1コマンド:

```sh
task up   -- neo4j
task seed -- neo4j           # task reset -- neo4j なら両方まとめて
```

## 既定値

| | 値 |
|---|---|
| ホスト | `localhost`（ループバックのみに公開） |
| ユーザー | `irodori` |
| パスワード | `irodori` |
| データベース | `samples` |

## エンジン一覧

ポートは `5xxxx` 帯。irodori-table 側のプレイグラウンドは `56xxx` 帯なので
同時に起動できる。

| エンジン | ポート | ユーザー | パスワード | DB | 投入方法 |
|---|--:|---|---|---|---|
| postgres | 55432 | irodori | irodori | samples | init フック |
| mysql | 55306 | irodori | irodori | samples | init フック |
| mariadb | 55307 | irodori | irodori | samples | init フック |
| mongodb | 57017 | irodori | irodori | samples | init フック |
| oracle | 55521 | irodori | irodori | `FREEPDB1` | init フック |
| clickhouse | 58123 | irodori | irodori | samples | init フック |
| timescaledb | 55433 | irodori | irodori | samples | init フック |
| cockroachdb | 55257 | `root` | *(なし)* | `defaultdb` | `task seed` |
| yugabytedb | 55434 | `yugabyte` | *(なし)* | `yugabyte` | `task seed` |
| tidb | 54000 | `root` | *(なし)* | `test` | `task seed` |
| sqlserver | 51433 | `sa` | `Irodori_Strong!23` | samples | `task seed` |
| redis | 56379 | — | irodori | `0` | `task seed` |
| neo4j | 57687 | `neo4j` | `irodoripass` | `neo4j` | `task seed` |
| memgraph | 57688 | — | *(なし)* | `memgraph` | `task seed` |
| cassandra | 59042 | — | *(なし)* | keyspace samples | `task seed` |
| scylladb | 59043 | — | *(なし)* | keyspace samples | `task seed` |
| questdb | 58812 | `admin` | `quest` | `qdb` | — |
| influxdb | 58086 | — | *(なし)* | samples | — |
| elasticsearch | 59200 | — | *(なし)* | index | — |
| openSearch | 59201 | — | *(なし)* | index | — |
| qdrant | 56333 | — | *(なし)* | collection | — |
| arangodb | 58529 | `root` | irodori | samples | — |
| dynamodb | 58000 | `irodori` | `irodori` | table | — |
| sqlite | — | — | — | `sqlite/samples.db` | `task seed` |
| duckdb | — | — | — | `duckdb/samples.duckdb` | `task seed` |

### 既定値から外れる5つとその理由

いずれも「警告」ではなく「拒否」してくるので、合わせるしかない。

- **SQL Server** — `sa` のパスワードが8文字以上かつ大小英数記号のうち3種を
  含まないと**起動自体を拒否**する。だから `Irodori_Strong!23`。
  証明書が自己署名なので `TrustServerCertificate=true` も必要。
- **Neo4j** — 8文字未満のパスワードを拒否するので `irodoripass`。
- **CockroachDB / YugabyteDB / TiDB** — insecure single-node 構成なので
  パスワードなし。CockroachDB は `sslmode=disable` の明示が必須。
- **QuestDB** — OSS 版にユーザー管理がなく `admin` / `quest` 固定。
- **Oracle** — `FREEPDB1` はスキーマではなくサービス名。その中のテーブルを
  `irodori` が持つ。

## 貼り付け用

`task urls` と同じ内容は [CONNECTIONS.md](CONNECTIONS.md#urls-to-paste) にある。

## 拡張が必要なエンジン

アプリ組み込み: postgres, mysql, mariadb, sqlite, timescaledb, cockroachdb,
yugabytedb, tidb, questdb, clickhouse, influxdb, snowflake

`legacy-connectors` またはマーケットプレイス拡張（「このデータソースは
このビルドでは利用できません」と出たら 設定 → 拡張 からインストール）:
sqlserver, oracle, mongodb, redis, neo4j, cassandra, scylladb, memgraph,
duckdb, elasticsearch, openSearch, qdrant, arangodb, dynamodb

## Web コンソール

| | URL |
|---|---|
| CockroachDB DB Console | http://localhost:55180 |
| Neo4j Browser | http://localhost:57474 |
| QuestDB Web Console | http://localhost:59000 |
| ArangoDB | http://localhost:58529 |
| YugabyteDB UI | http://localhost:55435 |
