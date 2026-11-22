---
type: Specification
title: "問題生成のコマンド"
description: "問題生成のコマンド `list_generated`（生成問題一覧）、`get_generation_options`（生成設定画面の要素の一覧と習熟レベル、初期値の材料）、`request_generation`（生成の依頼と `generation_id`）、`delete_generated` の引数と戻り値を定める。"
---

動作は [演習問題生成](../../exercise-generation/index.md) に従う。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。生成の経過と完了は、generate ジョブの `job_updated`（[ジョブのコマンドとイベント](jobs.md)）で知らせる。

## list_generated

生成問題一覧（[生成問題一覧と生成問題の演習](../../exercise-generation/list-and-exercise.md)）で呼ぶ。

- 引数: なし。
- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `items` | `object[]` | 生成問題。新しい順（`created_at` の新しい順）に並べる |
| `items[].meta` | [生成問題の保存](../../exercise-generation/storage.md) の `meta.json` | 生成問題のメタデータ |
| `items[].primary_elements` / `items[].secondary_elements` | `string[] \| null` | 生成問題の `lesson.json` の同名の項目（[レッスン定義の形式（lesson.json）](../../content-format/lesson-json.md)）。`lesson.json` が無いとき（`status` が `ready` でないとき）は `null` |

- 生成問題を開くときは、レッスン ID `gen-{generation_id}` で `open_lesson`・`get_lesson` を呼ぶ（[進捗のコマンド](progress.md)、[教材のコマンド](content.md)）。

## get_generation_options

生成設定画面（[演習問題の生成設定](../../exercise-generation/settings.md)、[問題生成の画面（生成問題一覧、生成設定画面）](../../screens/generation-screens.md)「生成設定画面」）を開いたときに呼ぶ。

- 引数: なし。
- 戻り値:

```json
{
  "chapters": [
    {
      "id": "ch04",
      "title": "所有権",
      "elements": [
        { "id": "ownership.move", "name": "所有権のムーブ", "description": "値の代入・関数呼び出しによる所有権の移動", "level": 1, "provisional": false }
      ]
    }
  ],
  "focus_weak_available": true,
  "focus_weak_elements": ["ownership.move", "borrow.mutable"],
  "recommended_difficulty": 2
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `chapters` | `object[]` | 要素を章ごとにまとめたもの。章の並びは学習順（[章とレッスンの並びの形式（chapters.json）](../../content-format/chapters-json.md)） |
| `chapters[].id` / `title` | `string` | 章の ID とタイトル |
| `chapters[].elements` | `object[]` | 要素マスタ（[要素マスタの形式（elements.json）](../../content-format/elements-json.md)）のうち、`chapter` がその章の要素 |
| `elements[].id` / `name` / `description` | `string` | 要素マスタの同名の項目 |
| `elements[].level` | `number`（整数） | 習熟レベル（[分析ファイル（analysis/analysis.md）](../../learning-record-and-analysis/analysis-file.md)「習熟レベル」）。分析ファイルに無い要素は 0（未学習） |
| `elements[].provisional` | `boolean` | 分析ファイルの `provisional`。分析ファイルに無い要素は `false` |
| `focus_weak_available` | `boolean` | 分析ファイルに要素が 1 件以上あれば `true`。`false` のとき、「苦手分野を中心にする」はオフに固定する |
| `focus_weak_elements` | `string[]` | 「苦手分野を中心にする」がオンのときの要素の初期値（[演習問題の生成設定](../../exercise-generation/settings.md)「要素の初期値」の規則で選んだもの。最大 5 件） |
| `recommended_difficulty` | `number`（整数） | 難易度の初期値（[演習問題の生成設定](../../exercise-generation/settings.md) の表） |

## request_generation

生成設定画面の「生成」で呼ぶ。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `settings` | [生成問題の保存](../../exercise-generation/storage.md) の `meta.json` の `settings` の形 | 生成設定（`focus_weak`・`elements`・`difficulty`・`message`） |

- 戻り値:

| 項目 | 型 | 内容 |
|---|---|---|
| `generation_id` | `string` | 作った生成問題の ID |
| `job_id` | `string` | 登録した generate ジョブの ID |

- `meta.json` の作成と generate ジョブの登録は [演習問題生成の流れ](../../exercise-generation/flow.md) に従う。
- エラー: `settings` が [演習問題の生成設定](../../exercise-generation/settings.md) の条件（要素は要素マスタにある最大 5 件、難易度は 1〜5、AI へのメッセージは最大 1,000 文字、`focus_weak_available` が `false` のときの「苦手分野を中心にする」はオフ）を満たさないときは `invalid_argument`。`claude` が見つからないときは `claude_unavailable`。

## delete_generated

生成問題一覧の「削除」で、確認ダイアログの後に呼ぶ。

- 引数: `generation_id`（`string`）。
- 戻り値: `null`。
- 消すものと残すものは [生成問題一覧と生成問題の演習](../../exercise-generation/list-and-exercise.md) に従う。
- エラー: その生成問題が無いときは `not_found`。
