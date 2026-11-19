# アーキテクチャ仕様書

アプリ全体の技術構成、フロントエンドとバックエンドの役割分担、外部ツールへの依存、データの保存場所を定める。

## 1. 技術構成

| 層 | 採用技術 |
|---|---|
| アプリ基盤 | Tauri 2 |
| フロントエンド | React + TypeScript（ビルドは Vite） |
| コードエディタ | Monaco Editor（シンタックスハイライトと基本的な編集機能のみ。補完・LSP 連携は行わない） |
| 動画の再生 | Remotion（`@remotion/player`）。動画はファイルに書き出さず、フロントエンドで描画する（`content-format.md` §5） |
| バックエンド | Rust（Tauri のコマンドとして実装） |
| 演習のビルド・実行 | 学習者の環境にインストールされた Rust ツールチェーン（`cargo`） |
| AI 連携 | 学習者の環境にインストールされた Claude Code（`claude -p`） |
| ナレーションの音声の作成（教材の作成時のみ） | VOICEPEAK（教材の作者の環境で使う。アプリには含めない。`content-format.md` §5.3） |

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

## 2. 役割分担

- フロントエンドはファイルシステムや子プロセスに直接アクセスしない。すべてバックエンドのコマンドを経由する。
- 動画のコードと音声（`content/lessons/{lesson_id}/video/` のうち `script.json` 以外のファイル。`content-format.md` §5.1）と、動画が読み込む画像（`assets/`）は、フロントエンドのビルドに含める。レッスンごとに分け、そのレッスンの動画を表示するときに読み込む（遅延読み込み）。フロントエンドは、これらをビルド済みのアセットとして読む。ビルド済みのアセットを読むことは、ファイルシステムへの直接のアクセスに当たらない。
- 台本（`script.json`）とレッスン定義（`lesson.json`）は、バックエンドがアプリ同梱データ（§4.2）から読む。フロントエンドは、レッスン定義と台本を `get_lesson` で受け取る。バックエンドは台本を質疑応答（`qa-chat.md` §3.1）に、レッスン定義をレッスン検索（`screens.md` §9）に使う。
- 集計・検証・ファイルの書き込みはバックエンドで行う。フロントエンドは表示と入力の受け付けに専念する。
- 時間のかかる処理（ビルド・実行・AI 呼び出し・ジョブ）は非同期とし、途中経過や完了はイベントでフロントエンドに通知する。

### 2.1 コマンドとイベント

コマンド・イベントの名前と担当範囲を以下に定める。入力・出力の詳細と動作は「担当仕様」の仕様書に従う。

**コマンド（フロントエンド → バックエンド）**

| 分類 | コマンド | 担当仕様 |
|---|---|---|
| 環境 | `get_environment_status`（cargo・claude の検出結果、初回準備の状態）、`redetect_tools` | 本書 §3 |
| 教材 | `list_chapters`、`get_lesson`（台本を含む）、`search_lessons` | `content-format.md`、`screens.md` |
| 進捗 | `get_progress`、`open_lesson`、`save_draft`、`reset_draft`、`get_passed_code`、`save_video_position` | `lesson-flow.md` |
| 実行 | `run_code`、`send_stdin`、`close_stdin`、`stop_run`、`submit_code` | `exercise-runner.md` |
| 質疑応答 | `send_chat`、`cancel_chat` | `qa-chat.md` |
| 問題生成 | `list_generated`、`request_generation`、`delete_generated` | `exercise-generation.md` |
| 学習記録・分析 | `get_dashboard`、`rebuild_analysis`、`reset_analysis`、`reset_all` | `learning-record-and-analysis.md`、`screens.md` |
| ジョブ | `list_jobs`、`retry_job` | `claude-integration.md` |
| 設定 | `get_settings`、`update_settings`、`open_in_os`、`test_claude_connection` | `screens.md`、`claude-integration.md` |

**イベント（バックエンド → フロントエンド）**

| イベント | 内容 | 担当仕様 |
|---|---|---|
| `run_output` | 実行中の出力（stdout / stderr / system） | `exercise-runner.md` |
| `run_finished` | 実行・提出の結果 | `exercise-runner.md` |
| `chat_delta` / `chat_finished` | 質疑応答の回答（ストリーミング） | `qa-chat.md` |
| `job_updated` | ジョブの状態変化 | `claude-integration.md` |
| `environment_updated` | ツール検出結果・初回準備の状態変化 | 本書 §3 |

## 3. 外部ツールへの依存

### 3.1 検出

アプリ起動時と、設定画面の「再検出」ボタンを押したときに検出する。

| ツール | 探索順 | 必要条件 |
|---|---|---|
| `cargo` / `rustc` | 設定で指定したパス → `PATH` → `~/.cargo/bin`（Windows: `%USERPROFILE%\.cargo\bin`） | `rustc --version` が 1.85 以上（edition 2024 が使えること） |
| `claude` | 設定で指定したパス → `PATH` → `~/.local/bin` → `~/.claude/local` → npm のグローバルインストール先（Windows は各対応パス） | `claude --version` が成功すること |

- `PATH` に見つかった場合も見つからなかった場合も、表の既定のインストール先を探索する（macOS で Finder から起動した場合を含む）。
- 検出結果（パス・バージョン）はメモリに保持し、`get_environment_status` で返す。

### 3.2 見つからない場合

| ツール | 動作 |
|---|---|
| `cargo` | 実行・提出ボタンを無効にし、演習画面とトップページに rustup の導入手順を案内する。動画の視聴などは利用できる |
| `claude` | AI 機能（質疑応答・問題生成）を無効にし、Claude Code の導入手順を案内する。学習記録・分析のジョブは登録だけ行い、`claude` が使えるようになってから実行する（`claude-integration.md` §2） |

アプリから Rust や Claude Code を自動でインストールすることはしない。

### 3.3 初回準備

初回起動時、および許可クレートの一覧が変わったとき（アプリの更新時）に、許可クレートのダウンロードとビルドをバックグラウンドで行う。詳細は `exercise-runner.md` §2 を参照。

## 4. データの保存場所

### 4.1 学習者データ（`{app_data_dir}`）

`{app_data_dir}` は Tauri の `app_data_dir()` とする。

```
{app_data_dir}/
├── settings.json                   # 設定（screens.md §8）
├── progress.json                   # レッスンの進捗（lesson-flow.md §3）
├── drafts/                         # 下書き（lesson-flow.md §5）
│   └── {lesson_id}.json
├── attempts/                       # 試行ログ（learning-record-and-analysis.md §3）
│   └── {session_id}.json
├── records/                        # 学習記録（learning-record-and-analysis.md §4）
│   └── {record_id}.md
├── analysis/                       # 分析ファイル（learning-record-and-analysis.md §5）
│   ├── analysis.md
│   └── history/
│       └── analysis_{YYYYMMDDTHHMMSS}.md
├── generated/                      # 生成問題（exercise-generation.md §5）
│   └── {generation_id}/
├── jobs.json                       # AI ジョブキュー（claude-integration.md §5）
├── workspace/                      # ビルド・実行用の作業領域（exercise-runner.md §2）
└── logs/                           # アプリのログ（non-functional.md §6）
```

### 4.2 アプリ同梱データ（`{app_resources}`）

読み取り専用。リポジトリ上の原本をビルド時にリソースとして同梱する。

```
{app_resources}/
├── content/                        # 教材（原本: リポジトリの content/。動画のコードと音声、動画が読み込む画像を除く（§2）。形式は content-format.md）
├── templates/                      # 学習記録・分析のテンプレート（原本: spec/templates/）
│   ├── record.md
│   └── analysis.md
├── prompts/                        # AI への指示文（原本: リポジトリの prompts/。claude-integration.md §4）
└── runner/                         # 許可クレートの定義（原本: リポジトリの runner/。exercise-runner.md §2）
    ├── allowed-crates.toml
    └── Cargo.lock
```

### 4.3 書き込みの規則

- JSON・Markdown ファイルの書き込みは、一時ファイルに書いてからリネームする。
- 同じファイルへの書き込みはバックエンド内で直列化する。
- アプリが管理する JSON ファイル（`settings.json`・`progress.json`・`jobs.json`）は `schema_version` を持つ。読み込めない場合は `.broken-{日時}` を付けてファイルを退避し、初期状態で作り直したうえで、ユーザーに通知する。

## 5. リポジトリ構成

```
progrust-app/
├── spec/                # 仕様書
├── content/             # 教材の原本（動画のコードと音声を含む。形式は content-format.md）
├── prompts/             # AI への指示文の原本
├── runner/              # 許可クレートの定義
├── tools/               # 教材チェックツール、音声を作るツールなどの開発用ツール（content-guidelines.md §6、content-format.md §5.3）
├── src/                 # フロントエンド
│   └── video/           # 動画の共通部品（プレーヤー・タイムライン・教材用の部品。content-format.md §5.5）
└── src-tauri/           # バックエンド
```

## 6. Tauri の権限

- フロントエンドに許可する Tauri の権限（capabilities）は、本アプリのコマンドの呼び出しと、外部 URL を OS の既定ブラウザで開くことに限る。ファイルシステム・シェルのプラグインはフロントエンドに公開しない。
- 外部サイトのページをアプリ内で表示しない。

## 7. 受け入れ条件

- フロントエンドのコードに、ファイルシステムや子プロセスを直接扱う処理がない。
- macOS で Finder から起動した場合でも、`~/.cargo/bin` にある `cargo` と `~/.local/bin` にある `claude` を検出できる。
- `cargo` が見つからない状態でも起動でき、動画を視聴できる。
- `claude` が見つからない状態でもレッスンを完了でき、完了時に記録ジョブが登録される。
- 書き込み中にプロセスを強制終了しても、`progress.json` などが壊れたファイルにならない。

## 8. 未決事項

- なし
