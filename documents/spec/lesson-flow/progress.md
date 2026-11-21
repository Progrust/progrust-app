---
type: Specification
title: "進捗（progress.json）"
description: "`progress.json` の形式と各フィールドの意味、動画の再生位置（`last_sentence_id`、`video_watched`）の保存と復元のしかた、動画のみのレッスンを完了にするときの値を定める。"
---

```json
{
  "schema_version": 1,
  "lessons": {
    "ch04-04": {
      "status": "completed",
      "first_completed_at": "2026-10-31T13:25:00+09:00",
      "last_completed_at": "2026-10-31T13:25:00+09:00",
      "completion_count": 1,
      "last_opened_at": "2026-10-31T13:02:00+09:00",
      "last_sentence_id": "m5",
      "video_watched": true,
      "last_passed_session_id": "20261031T130200_ch04-04"
    }
  }
}
```

| フィールド | 説明 |
|---|---|
| `status` | `in_progress`（開いたことがあり未完了）/ `completed`。一度も開いていないレッスンはキーごと存在しない（未着手） |
| `first_completed_at` / `last_completed_at` | 最初・最後に正解した日時。動画のみのレッスンでは、どちらも初めて動画を最後まで再生した日時 |
| `completion_count` | 正解した回数（やり直しを含む）。動画のみのレッスンでは 1 |
| `last_opened_at` | 最後に開いた日時 |
| `last_sentence_id` | 最後に再生していた文（[動画画面](../screens/video-screen.md) の再生中の文）の ID。まだ動画を再生していないときは `null` |
| `video_watched` | 動画を最後まで再生したことがあるか。「演習へ」の表示と、動画のみのレッスンの完了に使う（[レッスンの流れ](flow.md)） |
| `last_passed_session_id` | 最後に正解した試行セッションの ID。正解コードの参照に使う |

- 動画画面を開いたとき、`last_sentence_id` の文の頭で一時停止した状態で表示する。`last_sentence_id` が `null` のときと、その ID の文が台本（[動画の台本（script.json）](../content-format/video/script-json.md)）にないときは、動画の先頭から表示する。
- `last_sentence_id` と `video_watched` は、`save_video_position` で保存する。保存するのは、再生中の文が変わったとき・一時停止したとき・最後まで再生したとき・動画画面を離れるときとする。
- レッスン検索のモーダル（[レッスン検索](../screens/lesson-search.md)）での再生は、`last_sentence_id` と `video_watched` に記録しない。
- 章末復習レッスンでは、`last_sentence_id` は `null`、`video_watched` は `false` のままとする。
- 動画のみのレッスンでは、`video_watched` を初めて `true` にしたときに、`status` を `completed` にし、`first_completed_at`・`last_completed_at`・`completion_count` を記録する。その後に再生し直しても、これらは変えない。`last_passed_session_id` は持たない。
- 生成問題の進捗は `progress.json` ではなく、生成問題のメタデータに記録する（[生成問題の保存](../exercise-generation/storage.md)）。
- 完了済みのレッスンをやり直して途中でやめても、`status` は `completed` のままとする。
