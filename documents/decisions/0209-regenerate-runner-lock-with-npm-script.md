---
type: Decision
title: "ADR 0209"
description: "runner/Cargo.lock は、一時ディレクトリに allcrates/ と同じ形のライブラリを作って cargo generate-lockfile で生成する npm のスクリプト（npm run runner:lock）で作り直し、runner/ の 2 つのファイルは、教材と同じ仕組みで src-tauri/gen/bundled/runner/ に写して同梱する"
---

- 日付: 2026-11-23
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「許可クレート」、[開発環境と依存の版](../spec/architecture/development-environment.md)「開発で使うコマンド」、[データの保存場所](../spec/architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」
- 置き換え: なし

## 背景

`runner/Cargo.lock` は、`allcrates/` のライブラリをルートのパッケージにして生成する（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「許可クレート」）。仕様には、作り直す手順が無かった。また、`runner/` を `{app_resources}/runner/` に入れる方法も決まっていなかった。

この判断は、Claude が Issue #151 の実装で、技術的な判断として決めた。以下の理由は Claude が書いた。

## 決定

- `npm run runner:lock`（`tools/runner-lock/`）で、`runner/Cargo.lock` を作り直す。一時ディレクトリに、`allcrates/` と同じ形のライブラリ（パッケージ名 `progrust_allowed_crates`、edition 2024、`[dependencies]` は `allowed-crates.toml` のまま、空の `src/lib.rs`）を作り、`cargo generate-lockfile` を実行して、できた `Cargo.lock` を `runner/` に写す。
  - 理由: 作者が手でディレクトリを作る手順は、間違えやすい。`runner/` の中に `Cargo.toml` を置くと、`runner/` がそのまま cargo のパッケージに見え、仕様の「`runner/` は一覧と `Cargo.lock` だけ」という形が崩れる。一時ディレクトリなら、リポジトリに生成用のファイルが残らない。
  - 理由: スクリプトは、教材チェックツールなどと同じ `tools/` の Node のスクリプトにする。ほかの開発用のコマンドと同じく、`package.json` から呼べる。
- `runner/` の `allowed-crates.toml` と `Cargo.lock` は、ビルドスクリプト（`src-tauri/build.rs`）が `src-tauri/gen/bundled/runner/` に写し、`bundle.resources` がそのディレクトリを `{app_resources}/runner/` に入れる。写し方は教材と同じである（[ADR 0187](0187-stage-bundled-content-in-build-script.md)）。
  - 理由: 教材と同じ仕組みなら、開発時の起動でも、`cargo test`・`cargo clippy` だけの実行でも、写したディレクトリが先に用意される。`runner/` に将来ファイルが増えても、同梱するのは 2 つのファイルだけにできる。

## 検討した他の案

- 案: `bundle.resources` に `../runner/allowed-crates.toml` と `../runner/Cargo.lock` を並べる
  - 採用しなかった理由: 一覧の形では、`{app_resources}/_up_/runner/` の下に入る（[ADR 0187](0187-stage-bundled-content-in-build-script.md)「背景」）。
- 案: `Cargo.lock` の作り直しを、手順の文章だけにする
  - 採用しなかった理由: 手でディレクトリを作る手順は、版や edition を間違えやすい。コマンドにすれば、同じ結果が毎回できる。

## 影響

- クレートを追加・変更したら、`npm run runner:lock` を実行し、変わった `runner/Cargo.lock` をコミットする。
- `npm run runner:lock` は、cargo のレジストリの索引を使うので、ネットワークが要ることがある。
- `runner/Cargo.lock` が `allowed-crates.toml` と合っているかは、自動テストが確かめる。
