<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# サンプルクエリプロジェクト

Irodori Tableでの手動チェック用のエンジン固有クエリ。

```sh
make db-up DB=postgres
# Irodori Tableに接続し、projects/postgres/queries.sqlを実行
make db-down DB=postgres
```

カタログは `../db-feature-samples.json` です。