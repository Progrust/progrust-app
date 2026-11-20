---
type: Specification
title: "生成問題の保存"
description: "`generated/{generation_id}/` に保存するファイルの構成と、`meta.json` のフィールドおよび `status` の値を定める。"
---

`generation_id` は生成を依頼した日時 `YYYYMMDDTHHMMSS` とする。レッスン ID として扱うときは `gen-{generation_id}` とする。

```
generated/{generation_id}/
├── meta.json
├── lesson.json        # spec/content-format/lesson-json.md の形式（kind は generated）
└── exercise/          # spec/content-format/exercise.md の形式
```

`meta.json`:

```json
{
  "generation_id": "20261031T140000",
  "status": "ready",
  "settings": {
    "focus_weak": true,
    "elements": ["ownership.move"],
    "difficulty": 2,
    "message": ""
  },
  "created_at": "2026-10-31T14:00:00+09:00",
  "ready_at": "2026-10-31T14:02:10+09:00",
  "title": "在庫リストの集計",
  "summary": "所有権を意識しながらベクタを集計する",
  "error": null,
  "completion_count": 0,
  "last_completed_at": null,
  "last_passed_session_id": null
}
```

| フィールド | 説明 |
|---|---|
| `status` | `generating` / `ready` / `failed` |
| `error` | `failed` のときの理由（検証で失敗した項目など） |
| `completion_count` 〜 `last_passed_session_id` | [進捗（progress.json）](../lesson-flow/progress.md) の同名の項目と同じ意味 |
