<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# irodori-samples

Irodori Table 用のローカルデータベースを、1 つのクロスプラットフォーム・
ネイティブツールで起動・管理するサンプル集です。[go-task](https://taskfile.dev)、
対話型 TUI、直接実行する CLI は、すべて同じ Rust 製バイナリを利用します。

## できること

- 1 コマンドで DB の起動、準備完了待ち、必要なシード投入まで実行
- 対応する 25 エンジンを一覧・管理できる TUI
- 対応 DB 間で共通する決定的なサンプルデータ
- Windows、macOS、Linux でビルド・動作するネイティブ管理ツール
- ライフサイクル管理とフィクスチャ生成は Node.js に非依存

## クイックスタート

### 必要なもの

| 必要なもの | 必要になる場面 |
|---|---|
| go-task 3.x | README に記載した `task` コマンドの実行 |
| Rust 1.85 以降と Cargo | 初回ビルドと、ソース変更後の再ビルド |
| Docker Desktop または Podman | コンテナ型 DB エンジン |
| `sqlite3` または `duckdb` CLI | 対応する組み込み DB を使う場合のみ |
| OpenSSL | `task certs` と secure 構成を使う場合のみ |

初回ビルド後は `task doctor` で、コンテナランタイム、ネットワーク、
カタログ、エンジン固有の任意ツールをまとめて確認できます。

`doctor` はリポジトリ全体の準備状況を検証するため、Docker / Podman がない場合は
失敗として表示します。ただし、対応する CLI があれば SQLite と DuckDB 自体は
コンテナランタイムなしで利用できます。

### 初回起動

リポジトリのルートで実行します。

```sh
task doctor
task tui
```

ネイティブツールが必要なコマンドを初めて実行すると、最適化済みバイナリを
次の場所にビルドします。

- Windows: `.build/irodori/bin/irodori.exe`
- macOS / Linux: `.build/irodori/bin/irodori`

Rust ソースが変わるまでは同じバイナリを再利用します。ビルド後の実行には、
Rust ツールチェーンも Node.js も必要ありません。

TUI では矢印キーでエンジンを選び Enter を押します。起動、準備完了待ち、
必要なシード投入までを管理ツールがまとめて行います。

### CLI で起動する場合

スクリプトや CI では、TUI を使わず同じ管理ツールを実行できます。

```sh
task start -- postgres     # 起動、準備完了待ち、必要なシード投入
task status                # 全エンジンの状態を表示
task stop -- postgres      # データを残して停止
```

`--` 以降に引数を渡しにくい環境では、`DB=postgres` も利用できます。

```sh
task start DB=postgres
```

## 操作の選び方

| コマンド | 結果 | データ |
|---|---|---|
| `task start -- postgres` | 起動、準備完了待ち、必要なシード投入 | 保持 |
| `task up -- postgres` | Compose のみ起動。準備完了待ち・手動シードなし | 保持 |
| `task stop -- postgres` | エンジンを停止 | 保持 |
| `task seed -- redis` | 生成済みサンプルを投入または再作成 | 置換の場合あり |
| `task reset -- postgres` | データ削除後、利用可能な状態まで再作成 | 削除 |
| `task down -- postgres` | エンジンのリソースとローカルデータを削除 | 削除 |
| `task status` | コンテナ・組み込み DB の状態を表示 | 変更なし |
| `task logs -- postgres` | 直近のコンテナログを表示 | 変更なし |

> `seed`、`reset`、`down`、`down:all` は、ローカルのサンプルデータを
> 上書きまたは削除することがあります。TUI の reset / delete 操作には確認があります。

通常は、すぐ利用できる状態まで処理する `task start` を使います。
`task up` は低レベルの Compose 起動だけが必要な場合に使ってください。

## TUI の操作

| キー | 操作 |
|---|---|
| `Enter` | 起動、準備完了待ち、対応エンジンのシード投入 |
| `s` | データを残して停止 |
| `e` | シードを投入または再作成 |
| `r` | 確認後、データを削除して再作成 |
| `d` | 確認後、リソースとデータを削除 |
| `l` | 直近のログを表示 |
| `R` | 状態を再取得 |
| `q` | 終了 |

TUI は対話型ターミナルが必要で、Windows、macOS、Linux で動作します。

## コンテナランタイムと構成バリアント

管理ツールは Docker と Podman を検出します。両方が利用可能な場合は Podman を
優先します。Task 変数で明示的に選択できます。

```sh
task doctor RUNTIME=docker
task start DB=postgres RUNTIME=podman
```

用途別のバリアント:

```sh
task start:secure -- postgres  # ローカル TLS 証明書を使う secure 構成
task start:host -- postgres    # Linux の host-network fallback
```

secure コンテナ用のローカル CA と証明書は `task certs` で発行します。
管理ツール自体はクロスプラットフォームですが、各 DB ベンダーのコンテナイメージは
CPU アーキテクチャによって利用できない場合があります。

## 対応エンジン

| 分類 | エンジン |
|---|---|
| リレーショナル | `postgres` `mysql` `mariadb` `sqlite` |
| Enterprise SQL | `sqlserver` `oracle` |
| Distributed SQL | `cockroachdb` `yugabytedb` `tidb` |
| 時系列 | `timescaledb` `questdb` `influxdb` |
| カラム指向 | `clickhouse` `duckdb` |
| ドキュメント | `mongodb` |
| Key-value | `redis` `dynamodb` |
| グラフ | `neo4j` `memgraph` `arangodb` |
| Wide-column | `cassandra` `scylladb` |
| 検索 | `elasticsearch` `openSearch` |
| ベクトル | `qdrant` |

SQLite と DuckDB は組み込み DB なのでコンテナは不要です。それぞれの起動・
シード投入時にだけ、対応する CLI が必要です。

認証情報、ポート、接続 URL は [CONNECTIONS.ja.md](CONNECTIONS.ja.md) に
まとまっています。同じ URL 一覧をコマンドでも表示できます。

```sh
task urls
```

## サンプルデータとシード生成

シード対応エンジンには、既定で同じ決定的データセットが入ります。

| エンティティ | 行数 |
|---|---:|
| 顧客 | 200 |
| 商品 | 40 |
| 注文 | 600 |
| 注文明細 | 2,066 |
| イベント | 1,000 |

Unicode・双方向テキスト、NULL、正確な十進集計、JSON、リレーション、
エンジン固有型を含みます。

### シードの投入方法

- PostgreSQL、TimescaleDB、MySQL、MariaDB、MongoDB、Oracle、ClickHouse は
  イメージの初期化フックで読み込みます。
- CockroachDB、YugabyteDB、TiDB、SQL Server、Redis、Neo4j、Memgraph、
  Cassandra、ScyllaDB、SQLite、DuckDB は起動後にネイティブ管理ツールが投入します。
- その他のエンジンには、生成済みコマースデータではなく接続・機能確認用の
  フィクスチャがあります。

これらの違いは `task start` が自動的に処理します。

### コミット対象のフィクスチャを再生成

```sh
task generate              # 既定値: SCALE=0.02, SEED=20260807
task generate SCALE=0.1    # より大きな決定的データセット
task generate SEED=42      # 別の決定的データセット
```

生成処理も Rust 製ネイティブバイナリ内で動作し、コミット対象の
`<engine>/01_samples.*` を書き換えます。既定設定の出力はバイト単位で再現され、
単体テストで検証されています。

## ネイティブ実装と対応 OS

すべての入口が 1 つの実装を共有します。

| 入口 | 主な用途 |
|---|---|
| `task ...` | 短く、OSに依存しないプロジェクト操作 |
| `task tui` | 対話型のエンジン管理 |
| `.build/irodori/bin/irodori ...` | ビルド後の直接実行・自動化 |

ライフサイクル、ランタイム検出、シード投入、証明書生成、リポジトリ検証、
フィクスチャ生成、CLI 表示、TUI 状態管理は、`tools/irodori/` の Rust crate に
集約されています。Taskfile は薄いランチャーです。

このプロジェクトには Node.js の package manifest や Node.js 実行コマンドは
ありません。残っている MongoDB の `.js` は Mongo shell に渡す入力であり、
Node.js の実行時依存ではありません。

CI では Linux、macOS、Windows でネイティブツールをビルド・テストします。
各 OS で生成処理も実行し、コミット済みフィクスチャに差分が出ないことを検証します。

## 検証と開発

```sh
task list        # エンジンとシード方式を一覧表示
task build       # ネイティブツールを明示的にビルド
task test        # 単体テストとフィクスチャ再現テスト
task lint        # rustfmt と clippy
task check       # 全テストとサンプル・Compose・接続情報の検証
```

ビルド後、直接実行する CLI のヘルプを表示できます。

```sh
.build/irodori/bin/irodori help
```

Windows では末尾に `.exe` を付けてください。

## リポジトリ構成

- `<engine>/compose.yaml`: ローカル DB サービス。
- `<engine>/01_samples.*`: 対応エンジンの生成済みシード。
- `projects/<engine>/queries.*`: Irodori Table で試すクエリ。
- `tools/irodori/`: Rust 製の管理 CLI、TUI、シーダー、生成器。
- `generator/`: ネイティブ生成器が利用する静的 PostgreSQL デモデータ。
- `db-feature-samples.json`: 機械判読可能なサンプルカタログ。

## ライセンス

0BSD。本プロジェクトはほぼあらゆる目的で使用、コピー、改変、配布できます。
