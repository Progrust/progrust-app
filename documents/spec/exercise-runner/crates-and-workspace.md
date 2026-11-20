---
type: Specification
title: "許可クレートと作業領域"
description: "許可クレートの一覧（`runner/allowed-crates.toml`）、`workspace/` のディレクトリ構成、アプリ起動時の初回準備の条件・手順・準備中や失敗時の動作を定める。"
---

## 許可クレート

演習で使える外部クレートはリポジトリの `runner/allowed-crates.toml` に定める。

```toml
[dependencies]
rand = "=0.9.1"
```

- 書式は `Cargo.toml` の `[dependencies]` と同じとし、バージョンは `=` で固定する。
- この一覧から生成した `runner/Cargo.lock` もリポジトリに置き、アプリに同梱する。
- クレートを追加・変更したら `Cargo.lock` を作り直す。

## 作業領域

```
{app_data_dir}/workspace/
├── prepared.json      # 初回準備の状態
├── warmup/            # 初回準備用のプロジェクト（全許可クレートに依存）
├── project/           # 実行・提出のたびに組み立てるプロジェクト
├── validate/          # 生成問題の検証用プロジェクト（spec/exercise-generation/output-and-validation.md「検証」）
├── target/            # 許可クレートを使う演習のビルド先（初回準備と共有）
└── target-std/        # 許可クレートを使わない演習のビルド先
```

- ビルド先は `CARGO_TARGET_DIR` で指定する。許可クレートを使う演習は初回準備でビルド済みの依存を再利用できる。
- 許可クレートを使わない演習は `target-std/` にビルドし、初回準備中でも待たずにビルドできる。
- クレートのダウンロード先（レジストリのキャッシュ）は学習者の環境の既定（`CARGO_HOME`）を使う。

## 初回準備

**実行する条件**（アプリ起動時に判定する）

- `prepared.json` がない
- `prepared.json` の `crates_hash`（`allowed-crates.toml` と `Cargo.lock` のハッシュ値）が同梱のものと異なる
- `prepared.json` の `status` が `failed`

**手順**

1. `warmup/` に、全許可クレートに依存する `Cargo.toml`、同梱の `Cargo.lock`、空の `src/main.rs` を作る。
2. `cargo build` と `cargo test --no-run` を実行する（ネットワークが必要）。ビルド先は `target/`。
3. 成功したら `prepared.json` に `status: ready` と `crates_hash`・`prepared_at` を書く。失敗したら `status: failed` と エラー内容を書き、次回起動時に再度実行する。

**準備中・失敗時の動作**

- 許可クレートを使わない演習は通常どおり実行・提出できる。
- 許可クレートを使う演習は、実行・提出ボタンを無効にし、「クレートを準備しています」「クレートの準備に失敗しました（ネットワーク接続を確認してください）」と表示する。失敗時は再試行ボタンを表示する。
- 準備の状態は `environment_updated` イベントで通知する。
