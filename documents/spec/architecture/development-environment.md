---
type: Specification
title: "開発環境と依存の版"
description: "アプリの開発に要るもの（Node.js、npm、Rust ツールチェーン、Xcode Command Line Tools）、パッケージマネージャー（npm）、主な依存の版と版の固定のしかた（`package.json`・`package-lock.json`・`Cargo.lock`）、版を上げるときに確かめること、アプリの識別子（`com.progrust.desktop`）と開発時の識別子、lint と整形の道具、開発時の起動・本番ビルドなどの npm のコマンドの名前を定める。"
---

## 開発に要るもの

アプリは、作者の Mac（[対応環境](../non-functional/environments.md)）で開発し、ビルドする。次のものを入れる。

| もの | 版 | 用途 |
|---|---|---|
| Node.js | 25.9 系 | フロントエンドのビルド、Node.js で動く開発用ツール |
| npm | 11.12 系 | パッケージマネージャー（「パッケージマネージャー」） |
| Rust ツールチェーン（rustup の stable） | 学習者の環境に必要な rustc の最低バージョン以上（[対応環境](../non-functional/environments.md)） | バックエンドのビルドとテスト |
| Xcode Command Line Tools | 作者の Mac の macOS で入るもの | Tauri のビルド |

- バックエンドのビルドには、学習者のプログラムのビルドに使うもの（[外部ツール（cargo・claude）への依存](external-tools.md)）と同じ Rust ツールチェーンを使う。リポジトリに、ツールチェーンの版を固定するファイル（`rust-toolchain.toml`）を置かない。`src-tauri/Cargo.toml` の `rust-version` には、学習者の環境に必要な rustc の最低バージョンを書く。
- Claude Code は、AI 機能を開発で動かすときに使う。必要条件は、学習者の環境と同じく [外部ツール（cargo・claude）への依存](external-tools.md)「検出」に従い、開発のための最低の版は定めない。開発で動かした版は 2.1.329 である。
- 教材の作者だけに要るもの: VOICEPEAK と ffmpeg（ffprobe を含む）。使い方は [動画の音声](../content-format/video/voice.md)。アプリのビルドと実行には要らない。

## パッケージマネージャー

- JavaScript の依存は npm で入れる。yarn・pnpm・bun は使わない。
- `package.json` と `package-lock.json` は、リポジトリのルートに 1 組だけ置く。フロントエンドと、Node.js で動く開発用ツール（`tools/` の教材チェックツールなど）は、この 1 組の依存を使う。
- `package.json` の `engines` に、Node.js と npm の系列（上の表）を書く。
- 依存を入れるときは `npm ci` を使い、ロックファイルのとおりに入れる。依存を足すときと版を上げるときだけ、`npm install` でロックファイルを更新する。
- Rust の依存は cargo で入れる。`src-tauri/Cargo.lock` をリポジトリに入れる。
- `package-lock.json` と `src-tauri/Cargo.lock` は、どちらもリポジトリに入れる。

## 主な依存の版

| 依存 | 版 |
|---|---|
| `tauri`（crate）、`@tauri-apps/api`、`@tauri-apps/cli` | 2.12 系 |
| `react`、`react-dom` | 19.3 系 |
| `vite` | 8.3 系 |
| `typescript` | 6.0 系 |
| `monaco-editor` | 0.57 系 |
| `@monaco-editor/react` | 4.7 系 |
| `remotion` と `@remotion/` で始まるすべてのパッケージ | 4.0.545 |
| `eslint` | 10.15 系 |
| `typescript-eslint` | 8.73 系 |
| `stylelint` | 17.16 系 |

- 表の「系列」は、メジャーとマイナーの版（`2.12` など）を指す。`package.json` と `src-tauri/Cargo.toml` には、その系列の中のパッチの更新だけを許す範囲（`~2.12.7` の形）を書く。パッチの版は、ロックファイル（`package-lock.json`・`src-tauri/Cargo.lock`）を正とし、この文書に書かない。
- `remotion` と `@remotion/` で始まるパッケージは、すべて同じ版にし、パッチまで固定する。`package.json` には、範囲の記号（`^`・`~`）を付けずに表の版を書く。
- `tauri` の crate と `@tauri-apps/api`・`@tauri-apps/cli` は、同じ系列にそろえる。Tauri のプラグインを使うときは、`tauri-plugin-{名前}` の crate と `@tauri-apps/plugin-{名前}` を、互いに同じ系列にそろえる。
- 表にない依存（ほかのライブラリ、テストと整形の道具、間接の依存）は、入れたときの版をロックファイルで固定する。`package.json` と `Cargo.toml` の範囲は、npm と cargo の既定の書き方（`^`）でよい。
- 許可クレート（`runner/`）の版は、この文書の対象にしない（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)）。

## 版を上げるとき

- 表の依存の系列（Remotion はパッチの版）を上げるときは、同じ変更で、この文書の表と、`package.json`・`Cargo.toml`・ロックファイルを直す。
- 系列の中のパッチの更新は、表を直さずに、ロックファイルの更新だけで行ってよい。
- 版を上げたら（パッチの更新を含む）、次のことを確かめる。
  - コミット前の確認（[テストの方針](../non-functional/testing.md)「実行するとき」）が通る。
  - 本番ビルド（`npm run build`）で、[非機能要件の受け入れ条件](../non-functional/acceptance.md) の CSP の条件を満たす。
  - Tauri・React・Vite・Monaco Editor・Remotion の版を上げたときは、本番ビルドを人が操作して、動画の再生（音声つきの再生、シーク、テーマの切り替え）と、演習画面のエディタ（日本語の入力、ショートカット）が動くことを確かめる。Monaco Editor の版を上げたときは、あわせて、CDN を読まず、同梱の Web Worker で動くことを確かめる（[アプリの技術構成](tech-stack.md)）。

## アプリの識別子

| 項目 | 値 |
|---|---|
| Tauri の `identifier` | `com.progrust.desktop` |
| Tauri の `productName` | `Progrust` |
| 開発時の起動の `identifier` | `com.progrust.desktop.dev` |

- `{app_data_dir}`（[データの保存場所](data-locations.md)）は、作者の Mac では `~/Library/Application Support/com.progrust.desktop/` になる。
- 開発時の起動（`npm run dev`）は、開発時の識別子を使う。開発時の起動の `{app_data_dir}` は `~/Library/Application Support/com.progrust.desktop.dev/` になり、本番ビルドのアプリの学習者データを読み書きしない。開発時の識別子は、開発時の起動だけに重ねる設定のファイル（`src-tauri/tauri.dev.conf.json`）に書き、`npm run dev` が Tauri CLI の `--config` でこのファイルを渡す。

## lint と整形

| 対象 | lint | 整形 |
|---|---|---|
| TypeScript・TSX | ESLint（typescript-eslint）と、TypeScript の型検査（`tsc --noEmit`） | Prettier |
| CSS | stylelint | Prettier |
| Rust（`src-tauri/`） | Clippy（警告をエラーとして扱う） | rustfmt |

- 動画のコードの禁止事項の検査は、教材チェックツールの lint で行う（[動画のコードの禁止事項](../content-format/video/prohibited-code.md)）。アプリの lint と同じ ESLint・stylelint の版を使う。
- Clippy と rustfmt は、Rust ツールチェーンに付いているものを使う。
- デザインシステムの写し（[画面の見た目とテーマ](../screens/appearance.md)「デザインシステムの写し」）は、lint と整形の対象から外す。

## 開発で使うコマンド

開発で使うコマンドは、`package.json` の npm のスクリプトにする。

| コマンド | 内容 |
|---|---|
| `npm run dev` | 開発時の起動（`tauri dev --config src-tauri/tauri.dev.conf.json`。開発時の識別子を使う） |
| `npm run build` | 本番ビルド（`tauri build --bundles app`）。`.app` だけを作り、インストーラー（`.dmg` など）は作らない（[公開範囲と配布](../non-functional/distribution.md)）。できた `.app` は `src-tauri/target/release/bundle/macos/Progrust.app` に置かれる |
| `npm run lint` | lint と型検査（上の表の lint のすべて） |
| `npm run format` | 整形（上の表の整形のすべて）。`npm run format:check` は、整形が済んでいるかを確かめるだけで、ファイルを変えない |
| `npm test` | 自動テスト（Vitest と `cargo test`。[テストの方針](../non-functional/testing.md)） |
| `npm run check` | コミット前の確認。`npm run lint`、`npm run format:check`、`npm test` を順に実行する |

- Tauri が開発時の起動と本番ビルドの前に実行するフロントエンドだけのコマンド（`beforeDevCommand`・`beforeBuildCommand`）は、`npm run dev:web`（Vite の開発サーバー）と `npm run build:web`（Vite のビルド）にする。
