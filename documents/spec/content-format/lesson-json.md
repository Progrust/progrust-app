---
type: Specification
title: "レッスン定義の形式（lesson.json）"
description: "`lesson.json` のフィールド（`kind`・`tags`・`primary_elements`・`secondary_elements`・`estimated_minutes`・`exercise.judge`（`min_learner_tests` を含む）など）と、`kind` ごとの `tags`・要素・`exercise` の要否と主要素の件数、C のレッスンでのフィールドの値（要素が空、`layout` は `single`、`judge.mode` は `stdout`、対象は `main.c`）を定める。"
---

Rust のレッスンと生成問題の `lesson.json` は、次の形である。C のレッスンは「C のレッスン」に定める。

```json
{
  "id": "ch04-04",
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
| `kind` | `lesson` / `video_only`（動画のみのレッスン）/ `chapter_review` / `generated`（生成問題のみ） |
| `title` / `summary` | 表示名と、レッスン一覧に表示する 1 行の説明 |
| `tags` | 検索ワード（string[]）。レッスン検索の対象にする語（[レッスン検索](../screens/lesson-search.md)）。`kind` が `lesson`・`video_only` のレッスンでは必須で、1 件以上を書く。空の文字列と、同じレッスンの中での重複は書けない |
| `primary_elements` | 主要素の ID（Rust のレッスンと生成問題）。件数は、`lesson`・`generated` では 1〜2 件、`chapter_review` では 2〜5 件。`video_only` では空の配列にする。`lesson` では、[章とレッスンの一覧](../curriculum/chapters/index.md) の「主要素」と同じ要素を、同じ順に書く |
| `secondary_elements` | 副要素の ID（Rust のレッスンと生成問題）。`video_only` では空の配列にする |
| `estimated_minutes` | 目安の所要時間（分） |
| `exercise` | 演習の定義。`video_only` では書かない |
| `exercise.layout` | `single`（`src/main.rs` の 1 ファイルのみ）/ `multi`（複数ファイル） |
| `exercise.judge.mode` | `stdout` / `test`（[演習のファイル形式](exercise.md)「判定方式」） |
| `exercise.judge.target` | `test` 方式で隠しテストを組み込むファイル（`src/main.rs` または `src/lib.rs`）。`stdout` 方式では `src/main.rs` 固定 |
| `exercise.judge.min_learner_tests` | `test` 方式で、学習者が書いたテストを判定に含めるときの、必要なテストの件数（1 以上の整数）。省略時は学習者が書いたテストを判定に含めない（[コードの提出（submit）](../exercise-runner/submit.md)「学習者のテストの判定」）。`stdout` 方式では書けない |
| `exercise.dependencies` | 使用する許可クレートの名前（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「許可クレート」 の一覧にあるもの） |
| `exercise.readonly_files` | 学習者が編集できないファイルのパス（`multi` のみ） |
| `exercise.case_labels` | `test` 方式でテスト関数名に付ける表示名（例: `{"empty_input": "空の入力"}`）。省略時は関数名を表示する |

- 章末復習レッスン（`chapter_review`）と生成問題（`generated`）の `lesson.json` は、`tags` を持たない。

## C のレッスン

C の章（[章とレッスンの並びの形式（chapters.json）](chapters-json.md)）のレッスンの `lesson.json` は、上の表のうち次のフィールドの値が Rust のレッスンと違う。

- `kind` は `lesson`・`video_only`・`chapter_review` のどれか（意味は Rust と同じ）。`generated` は使わない。
- `primary_elements`・`secondary_elements` は、どの `kind` でも空の配列にする。C のレッスンは要素を持たない。
- `tags` は、Rust のレッスンと同じ決まりに従う（`lesson`・`video_only` で必須。`chapter_review` は持たない）。
- `exercise`（`video_only` では書かない）は次の値にする。

| フィールド | 値 |
|---|---|
| `exercise.layout` | `single` だけ。ファイルは `main.c` の 1 つ |
| `exercise.judge.mode` | `stdout` だけ |
| `exercise.judge.target` | `main.c` |
| `exercise.judge.min_learner_tests` | 書けない |
| `exercise.dependencies` | 空の配列 |
| `exercise.readonly_files` | 空の配列 |
| `exercise.case_labels` | 空のオブジェクト |

```json
{
  "id": "c02-05",
  "chapter": "c02",
  "kind": "lesson",
  "title": "ヒープと malloc・free",
  "summary": "malloc で確保したメモリを free で解放する流れを学ぶ",
  "tags": ["ヒープ", "malloc", "free", "メモリリーク"],
  "primary_elements": [],
  "secondary_elements": [],
  "estimated_minutes": 20,
  "exercise": {
    "layout": "single",
    "judge": { "mode": "stdout", "target": "main.c" },
    "dependencies": [],
    "readonly_files": [],
    "case_labels": {}
  }
}
```

- C の演習のファイルは [演習のファイル形式](exercise.md)「C の演習」に定める。
