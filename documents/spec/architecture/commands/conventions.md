---
type: Specification
title: "コマンドとイベントの共通の決まり"
description: "フロントエンドとバックエンドの間のコマンド（`invoke`）とイベント（`emit`）に共通する決まり（名前の付け方、引数と戻り値の JSON の書き方と型の表し方、日時の書き方、エラーの形（`code`・`message`・`detail`）とエラーの種類、AI 呼び出しのエラーの形、要求を見分ける `request_id`、イベントの宛先）を定める。"
---

コマンドとイベントの一覧と担当仕様は [フロントエンドとバックエンドの役割分担](../responsibilities.md)「コマンドとイベント」に、分類ごとの入力・出力の形はこのディレクトリの各文書に定める。

## 仕組み

- コマンドは、Tauri 2 のコマンドとしてバックエンドに実装し、フロントエンドは `invoke` で呼ぶ。戻り値は Rust の `Result` とし、成功は `Ok`、失敗は `Err` で返す。フロントエンドでは、`Ok` の値で `invoke` の Promise が解決し、`Err` の値で拒否される。
- イベントは、バックエンドが Tauri の `emit` で送り、フロントエンドは `listen` で受け取る。
- コマンドとイベントの名前は、英小文字と `_` で書く（snake_case）。コマンドは動詞から始める（例: `get_lesson`、`run_code`）。

## 引数と戻り値

- 引数・戻り値・イベントの内容は、どれも JSON のオブジェクトにする。各文書には、引数・戻り値・内容を、項目の表と、必要なら JSON の例で書く。
- 項目名は、教材やアプリのデータの JSON と同じく snake_case にする。Tauri 2 のコマンドの引数名は、既定では JavaScript 側で camelCase になるので、すべてのコマンドに `#[tauri::command(rename_all = "snake_case")]` を付け、フロントエンドも snake_case の名前で渡す（例: `invoke("get_lesson", { lesson_id: "ch04-04" })`）。
- 引数の無いコマンドは、引数を渡さずに呼ぶ。返す値の無いコマンドは `null`（Rust の `()`）を返す。
- 既に仕様に形があるデータ（`lesson.json`、`progress.json`、`settings.json` など）は、その仕様の形のまま返し、項目を書き写さない。各文書では、その仕様へのリンクで「〜の形」と参照する。

**型の表し方**

項目の表の「型」の列は、次のように書く。

| 書き方 | 意味 |
|---|---|
| `string`・`number`・`boolean` | JSON の文字列・数・真偽値。`number` のうち整数に限るものは「整数」と書き添える |
| `T[]` | `T` の配列 |
| `{ [key: string]: T }` | キーが文字列で値が `T` のオブジェクト（例: ファイルパスから内容への対応） |
| `T \| null` | `T` か `null`。項目そのものは省かない |
| `"a" \| "b"` | 文字列のうち、どれかの値 |
| 文書のリンク | リンク先の仕様が定める形 |

**日時**

- 日時は、`progress.json` などと同じく、時差を付けた ISO 8601 の文字列（例: `2026-10-31T13:25:00+09:00`）で表す。

## エラー

コマンドが失敗したとき（`Err`）は、次の形のオブジェクトを返す。

```json
{ "code": "busy", "message": "ほかの実行・提出が終わるまで待ってください。", "detail": null }
```

| 項目 | 型 | 内容 |
|---|---|---|
| `code` | `string` | エラーの種類（下の表の値） |
| `message` | `string` | 学習者にそのまま見せてよい日本語の説明 |
| `detail` | `string \| null` | 原因の詳しい内容（OS のエラーの文など）。無いときは `null` |

| `code` | 返すとき |
|---|---|
| `invalid_argument` | 引数が、型や仕様の条件（ファイルパスの条件、文字数の上限、値の範囲など）を満たさない |
| `not_found` | 指定したレッスン・生成問題・ジョブ・実行などが無い |
| `invalid_state` | いまの状態では、その操作ができない（各コマンドの文書に書いた場合） |
| `busy` | 同時に 1 つだけ行う処理（実行・提出、質疑応答）が、既に動いている |
| `cargo_unavailable` | `cargo` が見つからない（[外部ツール（cargo・claude）への依存](../external-tools.md)「見つからない場合」） |
| `clang_unavailable` | C の演習で、`clang` が見つからない（[C のツール（clang）への依存](../c-toolchain.md)「見つからない場合」） |
| `claude_unavailable` | `claude` が見つからない（[claude が使えない場合の動作](../../claude-integration/unavailable.md)） |
| `crates_not_ready` | 許可クレートを使う演習で、初回準備の状態が `ready` でない（[許可クレートと作業領域](../../exercise-runner/crates-and-workspace.md)「初回準備の状態」） |
| `io_error` | ファイルの読み書きに失敗した |
| `internal` | 上のどれでもない、想定外の失敗 |

- コマンドが受け付けた後に、時間のかかる処理（実行・提出・質疑応答）の中で起きた失敗は、`Err` ではなく、その処理の終わりのイベント（`run_finished`・`chat_finished`）で知らせる。

**AI 呼び出しのエラー**

AI の呼び出しの失敗（[AI 呼び出しのエラーの分類と表示](../../claude-integration/errors.md)）は、次の形で表す。`chat_finished` と `test_claude_connection` で使う。

```json
{ "category": "rate_limited", "detail": null }
```

| 項目 | 型 | 内容 |
|---|---|---|
| `category` | `"not_logged_in" \| "rate_limited" \| "timeout" \| "other"` | 分類。順に、[AI 呼び出しのエラーの分類と表示](../../claude-integration/errors.md)「分類」の未ログイン・利用上限・タイムアウト・その他に当たる |
| `detail` | `string \| null` | 「その他」の詳細（[AI 呼び出しのエラーの分類と表示](../../claude-integration/errors.md)「分類」の下の決まり）。ほかの分類では `null` |

- 未検出は、呼び出す前にコマンドが `claude_unavailable` を返す。出力不正は、質疑応答と接続テストでは起きない（[AI 呼び出しのエラーの分類と表示](../../claude-integration/errors.md)「分類」）。
- 画面に出す文は、`category` から [AI 呼び出しのエラーの分類と表示](../../claude-integration/errors.md)「分類」の「表示・動作」に従ってフロントエンドが決める。

## 要求を見分ける ID

実行・テスト実行・提出・質疑応答のように、コマンドで始めてイベントで経過と結果を受け取る処理では、その処理（要求）を `request_id` で見分ける。

- `request_id` は、フロントエンドが要求ごとに作る UUID（version 4）の文字列とし（`crypto.randomUUID()`）、処理を始めるコマンドの引数で渡す。
- バックエンドは、その要求について送るイベントのすべてに、同じ `request_id` を入れる。経過と結果を受け取る側は、`request_id` で自分の要求のイベントだけを選ぶ。
- 途中で操作するコマンド（標準入力、停止、回答の中断）も、`request_id` で対象を指す。
- フロントエンドは、イベントの受け取り（`listen`）を始めてから、処理を始めるコマンドを呼ぶ。バックエンドは、コマンドの戻り値より先にイベントを送ることがある。
- 動いている要求と同じ `request_id` が渡されたときは、`invalid_argument` を返す。

## イベントの宛先

- アプリのウィンドウは 1 つなので、イベントはすべて、そのウィンドウに送る（`emit`）。特定の要求に宛てたイベントも、`request_id` を入れて同じように送る。
- 状態の変化を知らせるイベント（`environment_updated`・`job_updated`）は、変化のたびに送る。フロントエンドは、画面を開いたときに取得のコマンド（`get_environment_status`・`list_jobs`）でいまの状態を読み、その後はイベントで更新する。
