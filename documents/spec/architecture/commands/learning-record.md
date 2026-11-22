---
type: Specification
title: "学習記録・分析のコマンド"
description: "学習記録・分析のコマンド `get_dashboard`（トップページの次にやるべきレッスン・完了数・章ごとの完了率・分析の要約と分析の状態 `none`・`ok`・`unreadable`）、`list_unreadable_records`、`rebuild_analysis`、`reset_analysis`、`reset_all` の引数と戻り値を定める。"
---

動作は [学習記録・分析](../../learning-record-and-analysis/index.md) と、[トップページ（ダッシュボード）](../../screens/top-page.md)・[設定画面と settings.json](../../screens/settings-screen.md) に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

## get_dashboard

トップページ（[トップページ（ダッシュボード）](../../screens/top-page.md)）を開いたときに呼ぶ。表示する値の選び方と並べ方はバックエンドが行い、フロントエンドは受け取った順に表示する。

- 引数: なし。
- 戻り値:

```json
{
  "next_lesson": {
    "lesson_id": "ch04-04", "chapter_id": "ch04", "chapter_title": "所有権",
    "title": "所有権のムーブ", "summary": "代入や関数呼び出しで所有権が移動する仕組みを学ぶ", "started": true
  },
  "lesson_counts": { "completed": 18, "total": 120 },
  "chapters": [ { "id": "ch02", "title": "変数と型", "completed": 6, "total": 6 } ],
  "analysis": {
    "state": "ok",
    "weak_elements": [ { "id": "ownership.move", "name": "所有権のムーブ", "level": 1, "provisional": false } ],
    "review_due_elements": [],
    "regressed_elements": [],
    "last_4_weeks": [5, 7, 3, 6],
    "summary_md": "所有権の…"
  }
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `next_lesson` | `object \| null` | 次にやるべきレッスン（[レッスンの受講順](../../lesson-flow/order.md)）。すべて完了していて無いときは `null` |
| `next_lesson.lesson_id` / `title` / `summary` | `string` | レッスン ID と、`lesson.json` の `title`・`summary` |
| `next_lesson.chapter_id` / `chapter_title` | `string` | レッスンが属する章の ID とタイトル |
| `next_lesson.started` | `boolean` | 進捗（[進捗（progress.json）](../../lesson-flow/progress.md)）があれば `true`。`false` は未着手（「始める」） |
| `lesson_counts` | `{ completed: number, total: number }` | 完了したレッスンの数と、全レッスンの数。どちらも準備中のレッスンを数えない（[トップページ（ダッシュボード）](../../screens/top-page.md)） |
| `chapters` | `{ id: string, title: string, completed: number, total: number }[]` | 章ごとの完了率に表示する章（[トップページ（ダッシュボード）](../../screens/top-page.md) の決まりで選んだ 9 章まで）。学習順に並べる |
| `analysis` | `object` | 分析に関する項目 |
| `analysis.state` | `"none" \| "ok" \| "unreadable"` | `none` は学習記録がまだない、`unreadable` は `analysis.md` を解析できない（[学習記録・分析ファイルの手動編集への対応](../../learning-record-and-analysis/manual-edit.md)）、`ok` はそれ以外 |
| `analysis.weak_elements` | 要素の要約 `[]` | 苦手分野。[トップページ（ダッシュボード）](../../screens/top-page.md) の決まりで選んで並べたもの |
| `analysis.review_due_elements` | 要素の要約 `[]` | 復習推奨（`review_due: true` の要素） |
| `analysis.regressed_elements` | 要素の要約 `[]` | 定着度が低下した要素（`regressed: true` の要素） |
| `analysis.last_4_weeks` | `number[]` | 学習ペース。`analysis.md` の `activity.last_4_weeks`（[分析ファイル（analysis/analysis.md）](../../learning-record-and-analysis/analysis-file.md)「frontmatter」） |
| `analysis.summary_md` | `string \| null` | 分析ファイルの本文の「総評」セクションの本文（Markdown） |

- 要素の要約は `{ id: string, name: string, level: number, provisional: boolean }` とする。`id`・`name` は要素マスタ（[要素マスタの形式（elements.json）](../../content-format/elements-json.md)）、`level`・`provisional` は分析ファイルの値。
- `analysis.state` が `ok` でないときは、要素の要約の配列を空、`last_4_weeks` を空、`summary_md` を `null` にする。

## list_unreadable_records

設定画面の「読み込めない学習記録」で呼ぶ（[学習記録・分析ファイルの手動編集への対応](../../learning-record-and-analysis/manual-edit.md)）。

- 引数: なし。
- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `records` | `{ file_name: string, reason: string }[]` | frontmatter を解析できず、集計から除外した学習記録。`file_name` は `records/` の中のファイル名、`reason` は解析できなかった理由 |

## rebuild_analysis

設定画面の「分析を再構築」で呼ぶ（[学習記録・分析の AI ジョブ](../../learning-record-and-analysis/ai-jobs.md)「分析の再構築（rebuild）」）。

- 引数: なし。
- 戻り値: `{ job_id: string }`。登録した rebuild ジョブの ID。

## reset_analysis

設定画面の「分析のみリセット」で、確認ダイアログの後に呼ぶ。

- 引数: なし。
- 戻り値: `null`。
- 動作は [設定画面と settings.json](../../screens/settings-screen.md)「設定項目」に従う。

## reset_all

設定画面の「学習履歴をすべてリセット」で、確認ダイアログの後に呼ぶ。

- 引数: なし。
- 戻り値: `null`。
- 消すものと残すものは [設定画面と settings.json](../../screens/settings-screen.md)「設定項目」に従う。
