---
type: Specification
title: "質疑応答で AI に渡す入力"
description: "動画の再生中と解説表示中のそれぞれで AI に渡す入力の内容と順序、画面に出すコード（`code_blocks`）の渡し方を定める。"
---

## 動画の再生中

1. 指示文（`prompts/chat-video.md`）
2. レッスンのタイトルと主要素の説明（要素マスタの `name`・`description`）。動画のみのレッスンには主要素がないので、タイトルだけを渡す
3. レッスンの台本（[動画の台本（script.json）](../content-format/video/script-json.md)）の内容
   - 全文: シーンごとにシーンの名前を見出しにし、そのシーンの各文の ID と `text` を台本の順に並べたもの
   - `ai_note`
   - 画面に出すコード（`code_blocks`。渡し方は「画面に出すコードの渡し方」）
4. 質問を送信したときに再生中だった文の ID と、その文の `text`
5. 既習の要素の一覧（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md) の定義による）
6. 会話履歴
7. 今回の質問

演習の問題文・模範解答・テストケースは渡さない。

## 解説表示中

1. 指示文（`prompts/chat-explanation.md`）
2. レッスン（または生成問題）のタイトルと主要素・副要素の説明
3. 問題文、解説、模範解答
4. 学習者が正解したときのコード
5. 会話履歴
6. 今回の質問

## 画面に出すコードの渡し方

台本の文は、コードの行を行番号で指すことがある（[動画の作成ルール](../content-guidelines/video.md)「コード」）。台本の `code_blocks` は、次のように渡す。

- 言語（`language`）とラベル（`label`）を添えたコードとして渡す。`language` が `text` のものは、ターミナルの出力として渡す。
- 行番号を表示するもの（[動画でのコードの表示](../content-format/video/code-display.md)）は、各行の先頭に、`code_blocks` の行番号を付けて渡す。
- Diff として表示するものは、Diff であることを示し、各行の先頭の `+` / `-` を残して渡す。行番号は変更後のファイルの行番号を付け、削除行には付けない。
