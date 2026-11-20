---
type: Specification
title: "データの保存場所"
description: "学習者データ（`{app_data_dir}`）とアプリ同梱データ（`{app_resources}`）のディレクトリ構成と、ファイルの書き込みの規則を定める。"
---

## 学習者データ（`{app_data_dir}`）

`{app_data_dir}` は Tauri の `app_data_dir()` とする。

```
{app_data_dir}/
├── settings.json                   # 設定（spec/screens/settings-screen.md）
├── progress.json                   # レッスンの進捗（spec/lesson-flow/progress.md）
├── drafts/                         # 下書き（spec/lesson-flow/drafts.md）
│   └── {lesson_id}.json
├── attempts/                       # 試行ログ（spec/learning-record-and-analysis/attempt-log.md）
│   └── {session_id}.json
├── records/                        # 学習記録（spec/learning-record-and-analysis/record-file.md）
│   └── {record_id}.md
├── analysis/                       # 分析ファイル（spec/learning-record-and-analysis/analysis-file.md）
│   ├── analysis.md
│   └── history/
│       └── analysis_{YYYYMMDDTHHMMSS}.md
├── generated/                      # 生成問題（spec/exercise-generation/storage.md）
│   └── {generation_id}/
├── jobs.json                       # AI ジョブキュー（spec/claude-integration/job-queue.md）
├── workspace/                      # ビルド・実行用の作業領域（spec/exercise-runner/crates-and-workspace.md）
└── logs/                           # アプリのログ（spec/non-functional/logging.md）
```

## アプリ同梱データ（`{app_resources}`）

読み取り専用。リポジトリ上の原本をビルド時にリソースとして同梱する。

```
{app_resources}/
├── content/                        # 教材（原本: リポジトリの content/。動画のコードと音声、動画が読み込む画像を除く（spec/architecture/responsibilities.md）。形式は spec/content-format/index.md）
├── templates/                      # 学習記録・分析のテンプレート（原本: リポジトリの documents/spec/learning-record-and-analysis/templates/ の record.md.tmpl と analysis.md.tmpl。同梱するときに .tmpl を取る）
│   ├── record.md
│   └── analysis.md
├── prompts/                        # AI への指示文（原本: リポジトリの prompts/。spec/claude-integration/prompts.md）
└── runner/                         # 許可クレートの定義（原本: リポジトリの runner/。spec/exercise-runner/crates-and-workspace.md）
    ├── allowed-crates.toml
    └── Cargo.lock
```

## 書き込みの規則

- JSON・Markdown ファイルの書き込みは、一時ファイルに書いてからリネームする。
- 同じファイルへの書き込みはバックエンド内で直列化する。
- アプリが管理する JSON ファイル（`settings.json`・`progress.json`・`jobs.json`）は `schema_version` を持つ。読み込めない場合は `.broken-{日時}` を付けてファイルを退避し、初期状態で作り直したうえで、ユーザーに通知する。
