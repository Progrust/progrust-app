---
type: Specification
title: "レッスンの完了"
description: "提出で正解したときにレッスンを完了とすること、動画のみのレッスンを動画の再生で完了とすること、`progress.json` の更新、試行ログと試行セッションの扱いの参照先を定める。"
---

- 提出で正解したとき、そのレッスンを完了とし、`progress.json` を更新する。
- 動画のみのレッスンは、動画を最後まで再生したとき（決め方は [レッスンの流れ](flow.md) の「演習へ」と同じ）に完了とし、`progress.json` を更新する（[進捗（progress.json）](progress.md)）。
- 試行ログへの `passed_at` の記録と、試行セッションの終了・記録ジョブの登録は [試行ログ（attempts/*.json）](../learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」 に従う。動画のみのレッスンには試行セッションがなく、記録ジョブも登録しない。
