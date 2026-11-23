---
type: Decision
title: "ADR 0213"
description: "「前回の正解コードを見る」では、最後に正解した試行セッションの最初に正解した `submit` イベントを返し、正解した `submit` イベントが無い試行ログでは `not_found` にする"
---

- 日付: 2026-11-24
- 関連仕様: [演習コードの下書き](../spec/lesson-flow/drafts.md)、[進捗のコマンド](../spec/architecture/commands/progress.md)「get_passed_code」
- 置き換え: なし

## 背景

Issue #156 で、`get_passed_code` を作った。仕様は、`last_passed_session_id` の試行ログのうち正解した `submit` イベントのファイルを返すとしていたが、次のことを決めていなかった。

- 1 つの試行ログに、正解した `submit` イベントが複数あるときに、どれを返すか。
- 試行ログはあるが、正解した `submit` イベントが無いときに、何を返すか。

どちらも、ふつうの操作では起きない。解説表示の状態では再び提出できず（[レッスンの流れ](../spec/lesson-flow/flow.md)）、`last_passed_session_id` には正解した提出を追記したセッションを書くからである。試行ログを手で書き換えたときなどに起きる。

## 決定

Claude が決めた。仕様の文は、関連仕様に書いた。

- 正解した `submit` イベントが複数あるときは、最初のものを返す。
  - 理由: 試行ログの `passed_at` は、同じセッションで再び正解しても最初に正解した日時を残す（[ADR 0212](0212-attempt-session-id-collision-and-append-target.md)）。返す日時（`submitted_at`）が `passed_at` と同じになる。
- 試行ログに正解した `submit` イベントが無いときは、試行ログが無いときと同じく `not_found` を返す。
  - 理由: 返すコードが無いことは、画面から見て、試行ログが無いときと変わらない。

## 検討した他の案

- 案: 正解した `submit` イベントが複数あるときは、最後のものを返す
  - 採用しなかった理由: 返す日時が `passed_at` と食い違う
- 案: 正解した `submit` イベントが無いときは、最後の `submit` イベントを返す
  - 採用しなかった理由: 正解していないコードを、正解のコードとして見せてしまう

## 影響

- 「前回の正解コードを見る」の日時は、そのセッションの `passed_at` と同じになる。
