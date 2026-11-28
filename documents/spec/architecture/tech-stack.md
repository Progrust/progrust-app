---
type: Specification
title: "アプリの技術構成"
description: "層ごとの採用技術（Tauri 2、React + TypeScript、Monaco Editor、react-markdown、Remotion、Rust、cargo、clang、Claude Code、VOICEPEAK）と、アプリの構成図を定める。"
---

| 層 | 採用技術 |
|---|---|
| アプリ基盤 | Tauri 2 |
| フロントエンド | React + TypeScript（ビルドは Vite） |
| コードエディタ | Monaco Editor（シンタックスハイライトと基本的な編集機能のみ。補完・LSP 連携は行わない）。シンタックスハイライトは、Rust の演習では Rust、C の演習では C の構文で行う。CDN を使わず、Web Worker を含めてアプリのビルドに同梱する。エディタの中に出る文言（右クリックのメニュー、検索・置換の欄、読み取り専用のエディタで文字を打ったときの知らせなど）は、Monaco Editor に同梱された日本語の翻訳で日本語にする |
| 教材の Markdown の表示 | react-markdown。問題文などの Markdown を、HTML の文字列を経ずに React の要素にする。生の HTML は表示せずに捨て、リンクは文字だけを表示する（押してもリンク先に移らない） |
| 動画の再生 | Remotion（`@remotion/player`）。動画はファイルに書き出さず、フロントエンドで描画する（[レッスンの動画の形式（video/）](../content-format/video/index.md)） |
| バックエンド | Rust（Tauri のコマンドとして実装） |
| 演習のビルド・実行 | Rust の演習は、学習者の環境にインストールされた Rust ツールチェーン（`cargo`）。C の演習は、学習者の環境にインストールされた Command Line Tools の `clang`（[C のツール（clang）への依存](c-toolchain.md)） |
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
     cargo・clang /                   claude -p
     学習者のプログラム
```
