---
type: Specification
title: "外部ツール（cargo・claude）への依存"
description: "cargo と claude の検出方法と必要条件、見つからない場合の動作、許可クレートの初回準備を定める。"
---

## 検出

アプリ起動時と、設定画面の「再検出」ボタンを押したときに検出する。

| ツール | 探索順 | 必要条件 |
|---|---|---|
| `cargo` / `rustc` | 設定で指定したパス → `PATH` → `~/.cargo/bin` | `rustc --version` が 1.85 以上（edition 2024 が使えること） |
| `claude` | 設定で指定したパス → `PATH` → `~/.local/bin` → `~/.claude/local` → npm のグローバルインストール先 | `claude --version` が成功すること |

- `PATH` に見つかった場合も見つからなかった場合も、表の既定のインストール先を探索する（macOS で Finder から起動した場合を含む）。
- 検出結果（パス・バージョン）はメモリに保持し、`get_environment_status` で返す。

## 見つからない場合

| ツール | 動作 |
|---|---|
| `cargo` | 実行・提出ボタンを無効にし、演習画面とトップページに rustup の導入手順を案内する。動画の視聴などは利用できる |
| `claude` | AI 機能（質疑応答・問題生成）を無効にし、Claude Code の導入手順を案内する。学習記録・分析のジョブは登録だけ行い、`claude` が使えるようになってから実行する（[claude が使えない場合の動作](../claude-integration/unavailable.md)） |

アプリから Rust や Claude Code を自動でインストールすることはしない。

## 初回準備

初回起動時、および許可クレートの一覧が変わったとき（アプリの更新時）に、許可クレートのダウンロードとビルドをバックグラウンドで行う。詳細は [許可クレートと作業領域](../exercise-runner/crates-and-workspace.md) を参照。
