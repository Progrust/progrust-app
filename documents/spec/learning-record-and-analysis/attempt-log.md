---
type: Specification
title: "試行ログ（attempts/*.json）"
description: "試行ログ `attempts/*.json` の試行セッションの開始・継続・終了の条件と、`schema_version`・`chat`・`run`・`submit` イベントを含む JSON のスキーマ、`run` の `exit_code` の意味、読み込めない試行ログの扱いを定める。"
---

演習中の行動をアプリが記録する生データ。学習記録の入力となる。
**記録を生成した後も削除せず保持する**。

## 試行セッションのライフサイクル

- **開始**: レッスンを開いたとき（動画画面。動画のない章末復習レッスン・生成問題では演習画面）。動画のみのレッスンでは、試行セッションを作らない。
  - 同じレッスンの試行ログが、同じ `session_id` で既にあるとき（同じ秒に前のセッションを終えて開き直したときなど）は、`session_id` の開始日時を 1 秒ずつ進めて、まだ無い ID にする。`started_at` は開いた日時のままにする。
- **継続**: 未完了のセッション（`ended_at` が無いセッション）があるレッスンを再度開いた場合は、同じセッションにイベントを追記する（アプリの再起動をまたいでも継続）。未完了のセッションが複数あるときは、最後に始めたものを続ける。
- **正解**: 提出で正解したとき、`passed_at` を記録する。同じセッションで再び正解しても、最初に正解した日時を残す。
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
  "schema_version": 1,
  "session_id": "20261031T130200_ch04-04",
  "lesson_id": "ch04-04",
  "kind": "lesson",
  "difficulty": null,
  "started_at": "2026-10-31T13:02:00+09:00",
  "passed_at": "2026-10-31T13:25:00+09:00",
  "ended_at": "2026-10-31T13:31:00+09:00",
  "events": [
    { "type": "chat", "at": "...", "context": "video:m5",
      "question": "ムーブ後に使えないのはなぜ？", "answer_summary": null },
    { "type": "run", "at": "...", "mode": "main",
      "files": { "src/main.rs": "..." },
      "result": { "status": "compile_error", "error_codes": ["E0382"],
                  "stdout": "", "stderr": "...", "exit_code": null, "stdin": "" } },
    { "type": "submit", "at": "...",
      "files": { "src/main.rs": "..." },
      "result": { "status": "wrong", "error_codes": [], "passed": 3, "total": 5,
                  "failed_cases": ["空の入力"], "stderr": "" } }
  ]
}
```

| フィールド | 説明 |
|---|---|
| `schema_version` | 試行ログの形式の版。いまは `1`（[データの互換性](../non-functional/data-compatibility.md)） |
| `session_id` | `{開始日時 YYYYMMDDTHHMMSS}_{lesson_id}` |
| `lesson_id` | レッスン ID。生成問題は `gen-{生成問題ID}` |
| `kind` | `lesson` / `chapter_review` / `generated` |
| `difficulty` | 生成問題の難易度（1〜5）。レッスンは `null` |

イベント種別:

| type | 記録タイミング | 主な内容 |
|---|---|---|
| `chat` | 質問を送信したとき | `context`（`video:{文の ID}` / `explanation`）、質問文。`answer_summary` は将来の拡張用で、現在は常に `null` |
| `run` | 実行・テスト実行が終わったとき | `mode`（実行は `main`、テスト実行は `test`。[コードの実行（run）](../exercise-runner/run.md)「テスト実行」）、全ファイルのスナップショット、実行結果、学習者が入力した標準入力（`stdin`） |
| `submit` | 提出の判定が終わったとき | 全ファイルのスナップショット、判定結果（`failed_cases` は失敗したテストケースの表示名） |

- 動画の視聴（再生・一時停止・シークなど）は試行ログに記録しない。

`result.status`: `run` は `success` / `compile_error` / `runtime_error` / `timeout` / `stopped`、`submit` は `compile_error` / `wrong` / `passed`（意味は [コードの実行（run）](../exercise-runner/run.md)「結果の状態」・[コードの提出（submit）](../exercise-runner/submit.md)「判定結果」）

- `run` の `result.exit_code` は、`run_finished` の `run.exit_code`（[実行のコマンドとイベント](../architecture/commands/run.md)「イベント」）と同じ値にする。学習者のプログラム（テスト実行では `cargo test`）の終了コードで、プログラムを起動していないとき（`compile_error` と、ビルド中に止めた `stopped`）と、シグナルで終わったとき（停止・時間切れ・出力量の上限で止めたときを含む）は `null` にする。
- スナップショットはファイル全体を保存する。
- `error_codes` の抽出方法と、`stdout` / `stderr` の保存の上限は [実行・提出の試行ログへの記録](../exercise-runner/attempt-log.md) に従う。
- 結果の状態が `crates_unavailable` の実行・提出は、学習者のコードを実行・判定していないので、記録しない（[実行・提出の試行ログへの記録](../exercise-runner/attempt-log.md)）。

## 読み込めない試行ログ

- 読み込めない場合（[データの保存場所](../architecture/data-locations.md)「書き込みの規則」と同じ条件）は、`.broken-{日時}` を付けて退避し、その試行ログは無いものとして扱う。試行ログは消さない決まりなので、作り直さない。
- 未完了のセッションを探すときに読み込めない試行ログがあっても、エラーにしない。そのレッスンに未完了のセッションが無ければ、新しいセッションを始める。
