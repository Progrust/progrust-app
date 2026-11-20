---
type: Specification
title: "試行ログ（attempts/*.json）"
description: "試行ログ `attempts/*.json` の試行セッションの開始・継続・終了の条件と、`chat`・`run`・`submit` イベントを含む JSON のスキーマを定める。"
---

演習中の行動をアプリが記録する生データ。学習記録の入力となる。
**記録を生成した後も削除せず保持する**。

## 試行セッションのライフサイクル

- **開始**: レッスンを開いたとき（動画画面。動画のない章末復習レッスン・生成問題では演習画面）。
- **継続**: 未完了のセッションがあるレッスンを再度開いた場合は、同じセッションにイベントを追記する（アプリの再起動をまたいでも継続）。
- **正解**: 提出で正解したとき、`passed_at` を記録する。
- **終了**: 正解後に以下のいずれかが起きたとき、`ended_at` を記録し記録ジョブを登録する。
  - 「レッスン一覧へ」「次のレッスンへ」「生成問題一覧へ」を選択した
  - 解説表示中の演習画面から別画面へ遷移した
  - アプリを終了した
  - アプリ起動時に `passed_at` があり `ended_at` がないセッションが見つかった（異常終了時の救済）
- 解説表示中の質問も、終了までは同じ試行セッションに記録される。
- 完了済みレッスンをやり直した場合は新しいセッションとする。

## スキーマ

```json
{
  "session_id": "20261031T130200_ch04-03",
  "lesson_id": "ch04-03",
  "kind": "lesson",
  "difficulty": null,
  "started_at": "2026-10-31T13:02:00+09:00",
  "passed_at": "2026-10-31T13:25:00+09:00",
  "ended_at": "2026-10-31T13:31:00+09:00",
  "events": [
    { "type": "chat", "at": "...", "context": "video:m5",
      "question": "ムーブ後に使えないのはなぜ？", "answer_summary": null },
    { "type": "run", "at": "...",
      "files": { "src/main.rs": "..." },
      "result": { "status": "compile_error", "error_codes": ["E0382"],
                  "stdout": "", "stderr": "...", "exit_code": 1, "stdin": "" } },
    { "type": "submit", "at": "...",
      "files": { "src/main.rs": "..." },
      "result": { "status": "wrong", "error_codes": [], "passed": 3, "total": 5,
                  "failed_cases": ["空の入力"], "stderr": "" } }
  ]
}
```

| フィールド | 説明 |
|---|---|
| `session_id` | `{開始日時 YYYYMMDDTHHMMSS}_{lesson_id}` |
| `lesson_id` | レッスン ID。生成問題は `gen-{生成問題ID}` |
| `kind` | `lesson` / `chapter_review` / `generated` |
| `difficulty` | 生成問題の難易度（1〜5）。レッスンは `null` |

イベント種別:

| type | 記録タイミング | 主な内容 |
|---|---|---|
| `chat` | 質問を送信したとき | `context`（`video:{文の ID}` / `explanation`）、質問文。`answer_summary` は将来の拡張用で、現在は常に `null` |
| `run` | 実行が終わったとき | 全ファイルのスナップショット、実行結果、学習者が入力した標準入力（`stdin`） |
| `submit` | 提出の判定が終わったとき | 全ファイルのスナップショット、判定結果（`failed_cases` は失敗したテストケースの表示名） |

- 動画の視聴（再生・一時停止・シークなど）は試行ログに記録しない。

`result.status`: `run` は `success` / `compile_error` / `runtime_error` / `timeout` / `stopped`、`submit` は `compile_error` / `wrong` / `passed`（意味は [コードの実行（run）](../exercise-runner/run.md)「結果の状態」・[コードの提出（submit）](../exercise-runner/submit.md)「判定結果」）

- スナップショットはファイル全体を保存する。
- `error_codes` の抽出方法と、`stdout` / `stderr` の保存の上限は [実行・提出の試行ログへの記録](../exercise-runner/attempt-log.md) に従う。
