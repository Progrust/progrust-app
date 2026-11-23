---
type: Decision
title: "ADR 0170"
description: "`progress.json` で、完了していないレッスンの完了のときに決まるフィールドを、省かずに `null`（`completion_count` は 0）で書く"
---

- 日付: 2026-11-23
- 関連仕様: [進捗（progress.json）](../spec/lesson-flow/progress.md)
- 置き換え: なし

## 背景

`progress.json` の例は完了したレッスンのものだけで、進行中のレッスンの、完了のときに決まるフィールド（`first_completed_at` など）を `null` で書くか、フィールドごと省くかが、未決事項として残っていた。実装計画の M1 で、進捗を読み書きするために決める必要があった。

## 決定

- どのフィールドも省かずに書き、まだ値が決まっていないフィールドは `null`、完了していないレッスンの `completion_count` は `0` にする（[進捗（progress.json）](../spec/lesson-flow/progress.md)）。動画のみのレッスンの `last_passed_session_id` も `null` のままにする。Claude が決めた。
  - 理由: コマンドの戻り値の決まり（[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)「型の表し方」の、`T | null` の項目そのものは省かないこと）と同じ書き方になり、`get_progress` などで `progress.json` の形のまま返せる。フィールドがあるかどうかと値が `null` かどうかの 2 通りを、読む側が区別しなくてよい。

## 検討した他の案

- 案: 値が決まっていないフィールドを省く
  - 採用しなかった理由: コマンドの戻り値の決まりと書き方が食い違い、読む側が「無い」と「`null`」の両方を扱う必要がある

## 影響

- 完了していないレッスンの進捗も、すべてのフィールドを持つ。
