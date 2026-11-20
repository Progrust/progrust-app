---
type: Specification
title: "ジョブキュー（jobs.json）"
description: "`jobs.json` の項目、ジョブの種類・状態・実行順・依存・リトライ・起動時の扱い・保持期間、UI への通知を定める。"
---

```json
[
  {
    "job_id": "job_20261031T133100_001",
    "type": "record",
    "session_id": "20261031T130200_ch04-03",
    "record_id": null,
    "generation_id": null,
    "status": "pending",
    "attempts": 0,
    "last_error": null,
    "created_at": "...",
    "updated_at": "..."
  }
]
```

| 項目 | 仕様 |
|---|---|
| `type` | `record` / `analysis` / `rebuild` / `generate` |
| 対象 | `record` は `session_id`、`analysis` は `record_id`、`generate` は `generation_id` を持つ |
| `status` | `pending` / `running` / `done` / `failed` |
| 実行順 | 1 件ずつ直列に実行する。`pending` のジョブから、優先度（`generate` > `record` > `analysis` > `rebuild`）の高いもの、同じ優先度では登録の古いものを選ぶ |
| 依存 | `analysis` は対応する `record` が `done` になってから登録する |
| リトライ | 失敗時は最大 3 回まで再実行（間隔 30 秒 → 2 分 → 10 分）。3 回失敗したら `failed` |
| `claude` 未検出時 | 実行せず `pending` のまま残す。`attempts` は増やさない |
| 起動時 | `running` のまま残っているジョブは `pending` に戻す |
| 保持 | `done` のジョブは 7 日後に削除 |
| UI | 状態が変わるたびに `job_updated` イベントを送る。ヘッダーに実行中のジョブを表示し、`failed` のジョブは設定画面に一覧表示して手動で再実行できる（[画面仕様](../screens/index.md)） |

- 質疑応答はジョブキューを通さず、ジョブの実行中でも並行して呼び出せる。
