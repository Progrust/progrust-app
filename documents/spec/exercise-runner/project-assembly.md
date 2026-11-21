---
type: Specification
title: "実行・提出時のプロジェクトの組み立て"
description: "実行・提出のたびに作る `project/`（と生成問題の検証の `validate/`）の `Cargo.toml`・`Cargo.lock`・`src/**`・`tests/**` の内容と、受け付けるファイルパスの条件、`--offline` やビルド先などのビルド条件を定める。"
---

実行・提出のたびに `project/` を空にしてから、以下を作る。

| ファイル | 内容 |
|---|---|
| `Cargo.toml` | パッケージ名 `progrust_exercise`、edition 2024。`[dependencies]` には、演習の `dependencies` に挙げたクレートを `allowed-crates.toml` の指定のまま書き、最後に `allcrates/` のライブラリへの path 依存 `progrust_allowed_crates = { path = "../allcrates" }` を書く（[許可クレートと作業領域](crates-and-workspace.md)「作業領域」）。許可クレートを使わない演習では `[dependencies]` を空にし、path 依存も書かない |
| `Cargo.lock` | 同梱の `Cargo.lock` をコピーする |
| `src/**`・`tests/**` | エディタ上の全ファイル |

- 演習のコードから `use` できるのは、演習の `dependencies` に挙げたクレートだけである。`progrust_allowed_crates` は中身が空なので、path 依存を足しても使えるクレートは増えない。
- エディタから受け取ったファイルパスは、`src/` 以下または `tests/` 以下で拡張子が `.rs` の相対パスに限る。`..` を含むパスや絶対パスは拒否する。
- ビルドには `--offline` を付け、`--locked` と `--frozen` は付けない。ビルドの失敗が [許可クレートと作業領域](crates-and-workspace.md)「初回準備のやり直しが要る失敗」に当てはまる場合は初回準備をやり直す。
- ビルド先は、許可クレートを使う演習は `target/`、使わない演習は `target-std/` にする（`CARGO_TARGET_DIR`）。
- 環境変数 `RUST_BACKTRACE=0` を設定する。

## 生成問題の検証（validate/）

生成問題の検証（[演習問題生成の出力と検証](../exercise-generation/output-and-validation.md)「検証」）では、`project/` と同じ方法で `validate/` を組み立てる。違うのはビルド先だけで、許可クレートを使う問題は `target-validate/`、使わない問題は `target-std-validate/` にする。パッケージ名は `project/` と同じ `progrust_exercise` にする。
