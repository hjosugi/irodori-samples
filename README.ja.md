<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# irodori-samples

Irodori Table用のローカルデータベースコンテナとサンプルクエリ。

## 使い方

```sh
make db-up DB=postgres
make db-verify DB=postgres
make db-down DB=postgres
```

スクリプトはPodmanが利用可能な場合はPodmanを、それ以外はDockerを使用します。

一般的な `DB` の値:

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

SQLiteとDuckDBは組み込みで、コンテナは不要です。

## ファイル

- `<engine>/compose.yaml`: 1つのデータベースコンテナ。
- `<engine>/01_samples.sql`: 利用可能な場合のシードデータ。
- `projects/<engine>/queries.*`: Irodori Tableで試すクエリ。
- `db-feature-samples.json`: 機械判読可能なサンプルカタログ。

ライセンス: `MIT OR 0BSD`。

## ライセンス

0BSD。本プロジェクトはほぼあらゆる目的で使用、コピー、改変、配布が可能です。