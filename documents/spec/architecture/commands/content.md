---
type: Specification
title: "教材のコマンド"
description: "教材のコマンド `list_chapters`（章とレッスンの一覧と、レッスンごとの `in_preparation`）、`get_lesson`（レッスン定義・台本・音声のディレクトリ・演習の内容。生成問題は `gen-{generation_id}` で開く）、`search_lessons`（検索の結果と強調する位置）の引数と戻り値を定める。"
---

教材の形式は [教材データ形式](../../content-format/index.md) に、画面での使い方は [画面仕様](../../screens/index.md) に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

## list_chapters

レッスン一覧・トップページなどで、章とレッスンの並びを読む。

- 引数: なし。
- 戻り値:

```json
{
  "chapters": [
    {
      "id": "ch04",
      "title": "所有権",
      "description": "Rust のメモリ管理の中心となる所有権の仕組みを学ぶ",
      "lessons": [
        {
          "id": "ch04-04",
          "kind": "lesson",
          "title": "所有権のムーブ",
          "summary": "代入や関数呼び出しで所有権が移動する仕組みを学ぶ",
          "estimated_minutes": 20,
          "in_preparation": false
        }
      ]
    }
  ]
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `chapters` | `object[]` | 章。並びは [章とレッスンの並びの形式（chapters.json）](../../content-format/chapters-json.md) の学習順 |
| `chapters[].id` / `title` / `description` | `string` | `chapters.json` の同名の項目 |
| `chapters[].lessons` | `object[]` | 章のレッスン。並びは `chapters.json` の `lessons` の順 |
| `lessons[].id` / `kind` / `title` / `summary` / `estimated_minutes` | | [レッスン定義の形式（lesson.json）](../../content-format/lesson-json.md) の同名の項目 |
| `lessons[].in_preparation` | `boolean` | 準備中のレッスンなら `true`（[章とレッスンの並びの形式（chapters.json）](../../content-format/chapters-json.md)「準備中のレッスン」） |

- 準備中のレッスンには `lesson.json` が無いので、`kind`・`title`・`summary`・`estimated_minutes` を `null` にする。


## get_lesson

レッスン（または生成問題）を表示するのに要るものを読む。動画画面・演習画面・レッスン検索のモーダルで使う。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `lesson_id` | `string` | レッスン ID。生成問題は `gen-{generation_id}`（[生成問題の保存](../../exercise-generation/storage.md)） |

- 戻り値（`lesson` と `script` の中身は省略して示す）:

```json
{
  "lesson": { "id": "ch04-04", "chapter": "ch04", "kind": "lesson", "title": "所有権のムーブ" },
  "script": { "scenes": [], "sentences": [] },
  "voice_dir": "/Applications/Progrust.app/Contents/Resources/content/lessons/ch04-04/video/voice",
  "exercise": {
    "problem_md": "## 問題\n...",
    "explanation_md": "## 考え方\n...",
    "starter_files": { "src/main.rs": "fn main() {\n    todo!()\n}\n" }
  }
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `lesson` | [レッスン定義の形式（lesson.json）](../../content-format/lesson-json.md) | レッスン定義。生成問題では `generated/{generation_id}/lesson.json` |
| `script` | [動画の台本（script.json）](../../content-format/video/script-json.md) `\| null` | 台本。動画の無いレッスン（章末復習レッスン・生成問題）では `null` |
| `voice_dir` | `string \| null` | 音声のディレクトリの絶対パス（[フロントエンドとバックエンドの役割分担](../responsibilities.md)）。動画の無いレッスンでは `null` |
| `exercise` | `object \| null` | 演習の内容。演習の無いレッスン（動画のみのレッスン）では `null` |
| `exercise.problem_md` | `string` | 問題文（[演習のファイル形式](../../content-format/exercise.md)「ファイル」の `problem.md`） |
| `exercise.explanation_md` | `string` | 解説（同じく `explanation.md`） |
| `exercise.starter_files` | `{ [path: string]: string }` | 初期コード。キーは cargo パッケージのルートからの相対パス（例: `src/main.rs`）、値はファイルの内容 |

- 判定に使うファイル（`cases.json`・`judge_tests.rs`）と `solution/` のファイルは返さない。判定はバックエンドが行う（[コード実行・判定](../../exercise-runner/index.md)）。模範解答のコードは、解説の `## 模範解答`（[演習のファイル形式](../../content-format/exercise.md)「解説（explanation.md）」）に入っている。
- エラー: レッスンが無いとき、準備中のレッスンのときと、生成問題の `meta.json` の `status` が `ready` でないときは `not_found`。

## search_lessons

ヘッダーの検索窓のレッスン検索（[レッスン検索](../../screens/lesson-search.md)）。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `query` | `string` | 検索窓に入力した文字列 |

- 戻り値:

```json
{
  "results": [
    {
      "lesson_id": "ch04-04",
      "chapter_id": "ch04",
      "chapter_title": "所有権",
      "title": "所有権のムーブ",
      "title_highlights": [{ "start": 3, "end": 6 }],
      "matched_tags": [{ "tag": "ムーブ", "highlights": [{ "start": 0, "end": 3 }] }]
    }
  ]
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `results` | `object[]` | 検索の結果。件数の上限と並び順は [レッスン検索](../../screens/lesson-search.md) に従う |
| `lesson_id` / `title` | `string` | レッスン ID とタイトル |
| `chapter_id` / `chapter_title` | `string` | レッスンが属する章の ID とタイトル |
| `title_highlights` | `{ start: number, end: number }[]` | タイトルの中で、検索の語に一致した範囲 |
| `matched_tags` | `{ tag: string, highlights: { start: number, end: number }[] }[]` | 一致した検索ワード（`lesson.json` の `tags`）と、その中で一致した範囲 |

- 範囲の `start`・`end`（整数）は、照合のための正規化（[レッスン検索](../../screens/lesson-search.md)）をかける前の、元の文字列の位置で表す。位置は JavaScript の文字列の添字（UTF-16 の単位）で数え、`end` は範囲の直後の位置とする。
- 語が無いとき（空白だけのときを含む）は、`results` を空にする。
