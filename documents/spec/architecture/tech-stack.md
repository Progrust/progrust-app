---
type: Specification
title: "アプリの技術構成"
description: "層ごとの採用技術（Tauri 2、React + TypeScript、Monaco Editor、Remotion、Rust、cargo、Claude Code、VOICEPEAK）と、アプリの構成図を定める。"
---

| 層 | 採用技術 |
|---|---|
| アプリ基盤 | Tauri 2 |
| フロントエンド | React + TypeScript（ビルドは Vite） |
| コードエディタ | Monaco Editor（シンタックスハイライトと基本的な編集機能のみ。補完・LSP 連携は行わない）。CDN を使わず、Web Worker を含めてアプリのビルドに同梱する |
| 動画の再生 | Remotion（`@remotion/player`）。動画はファイルに書き出さず、フロントエンドで描画する（[レッスンの動画の形式（video/）](../content-format/video/index.md)） |
| バックエンド | Rust（Tauri のコマンドとして実装） |
| 演習のビルド・実行 | 学習者の環境にインストールされた Rust ツールチェーン（`cargo`） |
| AI 連携 | 学習者の環境にインストールされた Claude Code（`claude -p`） |
| ナレーションの音声の作成（教材の作成時のみ） | VOICEPEAK（教材の作者の環境で使う。アプリには含めない。[動画の音声](../content-format/video/voice.md)） |

各技術の版、開発に要るもの、アプリの識別子は [開発環境と依存の版](development-environment.md) に定める。

```
┌─────────────────────── Tauri アプリ ──────────────────────┐
│  フロントエンド (React)                                   │
│   画面・状態管理・エディタ・動画の描画                    │
│        │ invoke（コマンド）        ▲ event（通知）        │
│        ▼                           │                      │
│  バックエンド (Rust)                                      │
│   ファイル入出力・集計・検証・ジョブキュー・プロセス管理  │
└───────┬───────────────────────────────┬───────────────────┘
        │ 子プロセス                    │ 子プロセス
        ▼                               ▼
     cargo / 学習者のプログラム       claude -p
```
