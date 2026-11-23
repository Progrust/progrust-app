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

- `content/` は、ビルドスクリプト（`src-tauri/build.rs`）が、リポジトリの `content/` から同梱するファイルだけを `src-tauri/gen/bundled/content/` に写し、Tauri がそれを `{app_resources}/content/` に入れる。写すときに、`.` で始まる名前のファイルとディレクトリ（`.DS_Store` など）も除く。
- 開発時の起動（`npm run dev`）でも、同じものを読む。Tauri のビルドが、同梱するファイルを実行ファイルの隣に写し、そこが `{app_resources}` になる。

```
{app_resources}/
├── content/                        # 教材（原本: リポジトリの content/。動画のコードと、動画が読み込む画像を除く。動画の音声は含める（spec/architecture/responsibilities.md）。形式は spec/content-format/index.md）
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
  - 一時ファイルは、書き込むファイルと同じディレクトリに、`.{ファイル名}.{プロセス ID}.tmp` の名前で置く。
- 同じファイルへの書き込みはバックエンド内で直列化する。
- アプリが管理する JSON ファイル（`settings.json`・`progress.json`・`jobs.json`）は `schema_version` を持つ。読み込めない場合は `.broken-{日時}` を付けてファイルを退避し、初期状態で作り直したうえで、ユーザーに通知する。
  - 読み込めない場合とは、JSON として読めない（UTF-8 でない場合を含む）、そのファイルの形に合わない、`schema_version` が無い、`schema_version` がアプリの読める版（いまの版と、[データの互換性](../non-functional/data-compatibility.md) で変換する古い版）でない、のどれかである。
  - ファイルの読み込みそのものに失敗した場合（権限が無いなど）は、退避せず、失敗として扱う（コマンドのエラーは `io_error`。[コマンドとイベントの共通の決まり](commands/conventions.md)「エラー」）。
  - ファイルが無い場合は、初期状態として扱う。退避も通知もせず、ファイルは次に書き込むときに作る。
  - `{日時}` は、退避したときの現地時刻の `YYYYMMDDTHHMMSS` である（例: `progress.json.broken-20261123T142500`）。同じ名前のファイルが既にある場合は、後ろに `-2`、`-3`…を付けて、既にあるファイルを上書きしない。
