---
type: Specification
title: "実行・提出時のプロジェクトの組み立て"
description: "実行・提出のたびに作る `project/` の `Cargo.toml`・`Cargo.lock`・`src/**` の内容と、受け付けるファイルパスの条件、`--offline` などのビルド条件を定める。"
---

実行・提出のたびに `project/` を空にしてから、以下を作る。

| ファイル | 内容 |
|---|---|
| `Cargo.toml` | パッケージ名 `progrust_exercise`、edition 2024。`[dependencies]` には演習の `dependencies` に挙げたクレートだけを `allowed-crates.toml` の指定のまま書く |
| `Cargo.lock` | 同梱の `Cargo.lock` をコピーする |
| `src/**` | エディタ上の全ファイル |

- エディタから受け取ったファイルパスは、`src/` 以下で拡張子が `.rs` の相対パスに限る。`..` を含むパスや絶対パスは拒否する。
- ビルドには `--offline` を付ける。依存が見つからないエラーになった場合は初回準備をやり直す（[許可クレートと作業領域](crates-and-workspace.md)「初回準備」）。
- 環境変数 `RUST_BACKTRACE=0` を設定する。
