---
type: Specification
title: "レッスン定義の形式（lesson.json）"
description: "`lesson.json` のフィールド（`kind`・`tags`・`primary_elements`・`secondary_elements`・`estimated_minutes`・`exercise.judge`（`min_learner_tests` を含む）など）と、`kind` ごとの `tags` の要否を定める。"
---

```json
{
  "id": "ch04-03",
  "chapter": "ch04",
  "kind": "lesson",
  "title": "所有権のムーブ",
  "summary": "代入や関数呼び出しで所有権が移動する仕組みを学ぶ",
  "tags": ["ムーブ", "move", "代入", "E0382"],
  "primary_elements": ["ownership.move"],
  "secondary_elements": ["fn.basic", "string.basic"],
  "estimated_minutes": 20,
  "exercise": {
    "layout": "single",
    "judge": { "mode": "stdout", "target": "src/main.rs" },
    "dependencies": [],
    "readonly_files": [],
    "case_labels": {}
  }
}
```

| フィールド | 説明 |
|---|---|
| `id` / `chapter` | レッスン ID と所属する章の ID。`chapters.json` と一致すること |
| `kind` | `lesson` / `chapter_review` / `generated`（生成問題のみ） |
| `title` / `summary` | 表示名と、レッスン一覧に表示する 1 行の説明 |
| `tags` | 検索ワード（string[]）。レッスン検索の対象にする語（[レッスン検索](../screens/lesson-search.md)）。`kind` が `lesson` のレッスンでは必須で、1 件以上を書く。空の文字列と、同じレッスンの中での重複は書けない |
| `primary_elements` | 主要素の ID（1〜2 件） |
| `secondary_elements` | 副要素の ID |
| `estimated_minutes` | 目安の所要時間（分） |
| `exercise.layout` | `single`（`src/main.rs` の 1 ファイルのみ）/ `multi`（複数ファイル） |
| `exercise.judge.mode` | `stdout` / `test`（[演習のファイル形式](exercise.md)「判定方式」） |
| `exercise.judge.target` | `test` 方式で隠しテストを組み込むファイル（`src/main.rs` または `src/lib.rs`）。`stdout` 方式では `src/main.rs` 固定 |
| `exercise.judge.min_learner_tests` | `test` 方式で、学習者が書いたテストを判定に含めるときの、必要なテストの件数（1 以上の整数）。省略時は学習者が書いたテストを判定に含めない（[コードの提出（submit）](../exercise-runner/submit.md)「学習者のテストの判定」）。`stdout` 方式では書けない |
| `exercise.dependencies` | 使用する許可クレートの名前（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「許可クレート」 の一覧にあるもの） |
| `exercise.readonly_files` | 学習者が編集できないファイルのパス（`multi` のみ） |
| `exercise.case_labels` | `test` 方式でテスト関数名に付ける表示名（例: `{"empty_input": "空の入力"}`）。省略時は関数名を表示する |

- 章末復習レッスン（`chapter_review`）と生成問題（`generated`）の `lesson.json` は、`tags` を持たない。
