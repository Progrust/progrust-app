---
type: Specification
title: "レッスンの完了"
description: "提出で正解したときにレッスンを完了とすることと、`progress.json` の更新、試行ログと試行セッションの扱いの参照先を定める。"
---

- 提出で正解したとき、そのレッスンを完了とし、`progress.json` を更新する。
- 試行ログへの `passed_at` の記録と、試行セッションの終了・記録ジョブの登録は [試行ログ（attempts/*.json）](../learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」 に従う。
