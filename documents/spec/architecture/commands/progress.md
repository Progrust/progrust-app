---
type: Specification
title: "進捗のコマンド"
description: "進捗のコマンド `get_progress`（進捗と推奨レッスン）、`open_lesson`（試行セッションの開始と進捗・下書きの読み込み）、`end_session`（画面の操作による試行セッションの終了）、`save_draft`・`reset_draft`、`get_passed_code`、`save_video_position` の引数と戻り値を定める。"
---

動作は [レッスン進行・進捗管理](../../lesson-flow/index.md) と [試行ログ（attempts/*.json）](../../learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

- 引数の `lesson_id` は、レッスン ID か、生成問題の `gen-{generation_id}`（[生成問題の保存](../../exercise-generation/storage.md)）とする。レッスンが無いときは `not_found` を返す。
- 生成問題の進捗は、`progress.json` ではなく `meta.json` にある（[進捗（progress.json）](../../lesson-flow/progress.md)）。

## get_progress

- 引数: なし。
- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `lessons` | [進捗（progress.json）](../../lesson-flow/progress.md) の `lessons` | レッスン ID から進捗への対応。`chapters.json` に無いレッスンの進捗は含めない（[データの互換性](../../non-functional/data-compatibility.md)） |
| `recommended_lesson_id` | `string \| null` | 推奨レッスン（[レッスンの受講順](../../lesson-flow/order.md)）の ID。無いときは `null` |

## open_lesson

レッスンを開いたとき（試行セッションの「開始」）に呼ぶ。レッスン検索のモーダルでの再生では呼ばない（[レッスン検索](../../screens/lesson-search.md)）。

- 引数: `lesson_id`（`string`）。
- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `session_id` | `string \| null` | 開始または継続した試行セッションの ID。動画のみのレッスンでは `null` |
| `progress` | [進捗（progress.json）](../../lesson-flow/progress.md) の `lessons` の 1 件 `\| null` | 更新した後の進捗。生成問題では `null` |
| `generated` | [生成問題の保存](../../exercise-generation/storage.md) の `meta.json` `\| null` | 生成問題の `meta.json`。レッスンでは `null` |
| `draft` | [演習コードの下書き](../../lesson-flow/drafts.md) の形 `\| null` | 下書き。無いときと、演習の無いレッスンでは `null` |

- 進捗の `status`・`last_opened_at` の更新は [進捗（progress.json）](../../lesson-flow/progress.md) に、試行セッションの開始と継続は [試行ログ（attempts/*.json）](../../learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」に従う。

## end_session

試行セッションの「終了」の条件のうち、画面の操作によるもの（「レッスン一覧へ」「次のレッスンへ」「生成問題一覧へ」を選んだ、解説表示中の演習画面から別の画面へ移った）が起きたときに呼ぶ（[試行ログ（attempts/*.json）](../../learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」）。

- 引数: `lesson_id`（`string`）。
- 戻り値: `null`。
- そのレッスンの試行セッションに `passed_at` があり `ended_at` が無いときに、セッションを終了する（`ended_at` の記録、記録ジョブの登録、下書きの削除（[演習コードの下書き](../../lesson-flow/drafts.md)））。それ以外のときは何もしない。
- アプリの終了と、起動時の救済による終了は、バックエンドが自分で行う。

## save_draft

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `lesson_id` | `string` | レッスン ID |
| `files` | `{ [path: string]: string }` | エディタ上の全ファイル（[演習コードの下書き](../../lesson-flow/drafts.md) の `files` と同じ形） |

- 戻り値: `null`。`updated_at` はバックエンドが付ける。
- 保存するときは [演習コードの下書き](../../lesson-flow/drafts.md) に従う。実行・提出の直前の保存は、`run_code`・`submit_code` が行う（[実行のコマンドとイベント](run.md)）。
- エラー: ファイルパスが [実行・提出時のプロジェクトの組み立て](../../exercise-runner/project-assembly.md) の条件を満たさないときは `invalid_argument`。

## reset_draft

「初期コードに戻す」で、確認ダイアログの後に呼ぶ。

- 引数: `lesson_id`（`string`）。
- 戻り値: 初期コードで上書きした後の下書き（[演習コードの下書き](../../lesson-flow/drafts.md) の形）。

## get_passed_code

「前回の正解コードを見る」で呼ぶ。

- 引数: `lesson_id`（`string`）。
- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `session_id` | `string` | `last_passed_session_id`（[進捗（progress.json）](../../lesson-flow/progress.md)。生成問題は `meta.json`） |
| `submitted_at` | `string` | 正解した `submit` イベントの `at` |
| `files` | `{ [path: string]: string }` | 正解した `submit` イベントのファイル（[試行ログ（attempts/*.json）](../../learning-record-and-analysis/attempt-log.md)「スキーマ」） |

- エラー: `last_passed_session_id` が無いときと、その試行ログが無いときは `not_found`。

## save_video_position

動画の再生位置を保存する。保存するときは [進捗（progress.json）](../../lesson-flow/progress.md) に従う。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `lesson_id` | `string` | レッスン ID |
| `last_sentence_id` | `string` | 再生中の文の ID |
| `video_watched` | `boolean` | 最後まで再生したら `true` |

- 戻り値: 更新した後の進捗（[進捗（progress.json）](../../lesson-flow/progress.md) の `lessons` の 1 件）。動画のみのレッスンを完了にしたときは、完了の項目も入る。
- `video_watched` は、一度 `true` になったら、`false` を渡しても `false` に戻さない。
- エラー: 動画の無いレッスンのときは `invalid_argument`。
