---
type: Specification
title: "フロントエンドとバックエンドの役割分担"
description: "フロントエンドがファイルシステムや子プロセスに直接触れないことなど両者の役割分担と、コマンド・イベントの名前と担当仕様を定める。"
---

- フロントエンドはファイルシステムや子プロセスに直接アクセスしない。すべてバックエンドのコマンドを経由する。
- 動画のコードと音声（`content/lessons/{lesson_id}/video/` のうち `script.json` 以外のファイル。[動画を構成するファイル](../content-format/video/structure.md)）と、動画が読み込む画像（`assets/`）は、フロントエンドのビルドに含める。レッスンごとに分け、そのレッスンの動画を表示するときに読み込む（遅延読み込み）。フロントエンドは、これらをビルド済みのアセットとして読む。ビルド済みのアセットを読むことは、ファイルシステムへの直接のアクセスに当たらない。
- 台本（`script.json`）とレッスン定義（`lesson.json`）は、バックエンドがアプリ同梱データ（[データの保存場所](data-locations.md)「アプリ同梱データ（`{app_resources}`）」）から読む。フロントエンドは、レッスン定義と台本を `get_lesson` で受け取る。バックエンドは台本を質疑応答（[質疑応答で AI に渡す入力](../qa-chat/ai-input.md)「動画の再生中」）に、レッスン定義をレッスン検索（[レッスン検索](../screens/lesson-search.md)）に使う。
- 集計・検証・ファイルの書き込みはバックエンドで行う。フロントエンドは表示と入力の受け付けに専念する。
- 時間のかかる処理（ビルド・実行・AI 呼び出し・ジョブ）は非同期とし、途中経過や完了はイベントでフロントエンドに通知する。

## コマンドとイベント

コマンド・イベントの名前と担当範囲を以下に定める。入力・出力の詳細と動作は「担当仕様」の仕様書に従う。

**コマンド（フロントエンド → バックエンド）**

| 分類 | コマンド | 担当仕様 |
|---|---|---|
| 環境 | `get_environment_status`（cargo・claude の検出結果、初回準備の状態）、`redetect_tools` | 本書 [外部ツール（cargo・claude）への依存](external-tools.md) |
| 教材 | `list_chapters`、`get_lesson`（台本を含む）、`search_lessons` | [教材データ形式](../content-format/index.md)、[画面仕様](../screens/index.md) |
| 進捗 | `get_progress`、`open_lesson`、`save_draft`、`reset_draft`、`get_passed_code`、`save_video_position` | [レッスン進行・進捗管理](../lesson-flow/index.md) |
| 実行 | `run_code`、`send_stdin`、`close_stdin`、`stop_run`、`submit_code` | [コード実行・判定](../exercise-runner/index.md) |
| 質疑応答 | `send_chat`、`cancel_chat` | [質疑応答](../qa-chat/index.md) |
| 問題生成 | `list_generated`、`request_generation`、`delete_generated` | [演習問題生成](../exercise-generation/index.md) |
| 学習記録・分析 | `get_dashboard`、`rebuild_analysis`、`reset_analysis`、`reset_all` | [学習記録・分析](../learning-record-and-analysis/index.md)、[画面仕様](../screens/index.md) |
| ジョブ | `list_jobs`、`retry_job` | [Claude Code 連携](../claude-integration/index.md) |
| 設定 | `get_settings`、`update_settings`、`open_in_os`、`test_claude_connection` | [画面仕様](../screens/index.md)、[Claude Code 連携](../claude-integration/index.md) |

**イベント（バックエンド → フロントエンド）**

| イベント | 内容 | 担当仕様 |
|---|---|---|
| `run_output` | 実行中の出力（stdout / stderr / system） | [コード実行・判定](../exercise-runner/index.md) |
| `run_finished` | 実行・提出の結果 | [コード実行・判定](../exercise-runner/index.md) |
| `chat_delta` / `chat_finished` | 質疑応答の回答（ストリーミング） | [質疑応答](../qa-chat/index.md) |
| `job_updated` | ジョブの状態変化 | [Claude Code 連携](../claude-integration/index.md) |
| `environment_updated` | ツール検出結果・初回準備の状態変化 | 本書 [外部ツール（cargo・claude）への依存](external-tools.md) |
