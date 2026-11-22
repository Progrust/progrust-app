---
type: Specification
title: "ジョブのコマンドとイベント"
description: "ジョブの形（`jobs.json` のジョブの項目と、対象の表示名 `target_title`）と、ジョブのコマンド `list_jobs`・`retry_job` の引数と戻り値、イベント `job_updated` の内容を定める。"
---

動作は [ジョブキュー（jobs.json）](../../claude-integration/job-queue.md) に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

## ジョブの形

`list_jobs`・`retry_job` の戻り値と `job_updated` の内容では、ジョブを次の形で表す。

| 項目 | 型 | 内容 |
|---|---|---|
| `job_id` 〜 `updated_at` | [ジョブキュー（jobs.json）](../../claude-integration/job-queue.md) のジョブの項目 | `jobs.json` の `jobs` の 1 件の項目をそのまま入れる |
| `target_title` | `string \| null` | ジョブの対象の表示名。`record` は試行セッションのレッスン（生成問題）のタイトル、`analysis` は学習記録の `title`、`generate` は生成問題の `title`（決まる前は `null`）、`rebuild` は `null` |

- ヘッダーのジョブの一覧と、設定画面の「失敗したジョブ」（種類・対象・エラー内容）は、この形から表示する（[全画面に共通するレイアウト](../../screens/common-layout.md)、[設定画面と settings.json](../../screens/settings-screen.md)「設定項目」）。

## list_jobs

- 引数: なし。
- 戻り値: `{ jobs: ジョブ[] }`。`jobs.json` にあるジョブを、登録の古い順に並べる。

## retry_job

設定画面の「失敗したジョブ」の再実行ボタンで呼ぶ。

- 引数: `job_id`（`string`）。
- 戻り値: 再実行を受け付けた後のジョブ。
- 状態を `failed` から `pending` に戻し、実行の順番は [ジョブキュー（jobs.json）](../../claude-integration/job-queue.md) に従う。
- エラー: そのジョブが無いときは `not_found`。状態が `failed` でないときは `invalid_state`。

## イベント

**`job_updated`**

- 内容: `{ job: ジョブ }`。状態が変わった後のジョブ。
- ジョブを登録したときと、ジョブの状態が変わるたびに送る（[ジョブキュー（jobs.json）](../../claude-integration/job-queue.md) の「UI」）。
- 問題生成の完了・失敗とジョブの失敗の通知（[画面の通知（トースト）](../../screens/notifications.md)）は、フロントエンドがこのイベントから出す。生成問題の完了は、`type` が `generate` のジョブの `status` が `done` になったことで分かる。生成問題を開くときのレッスン ID は `gen-{generation_id}` とする。
