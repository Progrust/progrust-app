---
type: Specification
title: "教材チェックツール"
description: "`tools/` の教材チェックツールで検証するチェック項目（エラーとして扱うもの）と、準備中のレッスンの扱い、警告として扱う項目（動画の長さ、文の長さ、読み）を定める。"
---

`tools/` に教材チェックツールを用意し、教材を追加・変更したら実行する。CI でも実行する。実行するコマンドは `npm run content:check`（[開発環境と依存の版](../architecture/development-environment.md)「開発で使うコマンド」）とする。動画のコードの型は、このツールでは検査せず、`npm run lint` の型検査で検査する（[開発環境と依存の版](../architecture/development-environment.md)「lint と整形」）。

| チェック項目 | 内容 |
|---|---|
| カリキュラム | `chapters.json` の章の並び、各章の ID・タイトル・説明、レッスンの並びと、章末復習レッスン以外の各レッスンの `lesson.json` の `title` が [章とレッスンの一覧](../curriculum/chapters/index.md) と一致する。`lesson.json` の `kind` が `video_only` のレッスンが、同じ一覧で動画のみのレッスンと定めたレッスンと一致する。`kind` が `lesson` の各レッスンの `lesson.json` の `primary_elements` が、同じ一覧の「主要素」と一致する |
| 構成 | `chapters.json` の全レッスンのディレクトリと必要なファイルが存在する。章末復習レッスン以外は、`video/` に `script.json`・`index.tsx`・`durations.json` と、すべての文の音声ファイルがある。動画のみのレッスンには `exercise/` がない |
| 定義 | `lesson.json` が [レッスン定義の形式（lesson.json）](../content-format/lesson-json.md) の形式に合っている。要素 ID が要素マスタに存在する |
| 既習要素 | `secondary_elements` の各要素が、そのレッスンにとって既習の要素である |
| 要素マスタ | `deprecated` でない要素は、いずれかのレッスンの `primary_elements` に含まれている |
| 要素と一覧 | 要素マスタの `id` が [要素の分け方と要素 ID の付け方](elements.md)「要素 ID」の形式に合い、重複がない。[章とレッスンの一覧](../curriculum/chapters/index.md) の「主要素」の要素 ID が、すべて要素マスタにあり、`deprecated` でない。`deprecated` でない要素は、一覧のいずれかのレッスンの「主要素」にあり、`chapter` が、一覧の学習順で最初にその要素を「主要素」とするレッスンの章と一致する |
| 章末復習の主要素 | 章末復習レッスンの `primary_elements` の各要素が、[章とレッスンの一覧](../curriculum/chapters/index.md) で、その章のレッスンの「主要素」になっている |
| 台本 | `script.json` が [動画の台本（script.json）](../content-format/video/script-json.md) の形式に合っている（ID の重複がないこと、文の並び、`say` が 140 字以下であることを含む）。`code_blocks` が [動画でのコードの表示](../content-format/video/code-display.md) の行番号と Diff の決まりに合っている。`index.tsx` のシーンが台本の `scenes` と一致する |
| 音声 | すべての文に音声ファイルがあり、`durations.json` の `say`・`voice` が、台本の `say`・`content/voice.json` と同じである。台本にない文の音声ファイルと記録がない |
| lint | 動画のコードが [動画のコードの禁止事項](../content-format/video/prohibited-code.md) に違反していない |
| 描画 | すべてのシーンをエラーなく描画できる。シーンごとに、最初のフレーム、各文の開始位置のフレーム、最後のフレームを描画する。台本にない文やほかのシーンの文を指す合図（[動画のタイムラインと合図](../content-format/video/timeline-and-cues.md)）と、制約に違反する `CodeBlock` の指定（[動画でのコードの表示](../content-format/video/code-display.md)）は、ここでエラーになる。描画時の例外と、ブラウザの console に出たエラーを、エラーにする（console の警告はエラーにしない）。見つかるのは、描画したフレームで実行された処理のエラーだけである。CI では、毎回すべてのレッスンを描画する |
| 表紙 | 最初のシーンが `LessonTitle` を 1 つ表示する（[動画のシーンの書き方](../content-format/video/scenes.md)）。最初のシーンの、「描画」で描画したフレームごとに、表示している `LessonTitle` を数える。どのフレームでも表示していないときと、2 つ以上表示しているフレームがあるときに、エラーになる |
| 問題文・解説 | [演習のファイル形式](../content-format/exercise.md)「問題文（problem.md）」・[演習のファイル形式](../content-format/exercise.md)「解説（explanation.md）」 の必須見出しがある |
| 模範解答 | 模範解答が判定ですべてのテストケースに通る（[コード実行・判定](../exercise-runner/index.md) の判定処理を使う） |
| 初期コード | 初期コードが判定で正解にならない |
| テストケース | [演習の作成ルール](exercise.md)「テストケース」に定めた件数以上ある。`test` 方式では `case_labels` のキーが `judge_tests.rs` のテスト関数名と一致する |
| クレート | `dependencies` が許可クレートの一覧にある |

「問題文・解説」「模範解答」「初期コード」「テストケース」「クレート」は、動画のみのレッスンでは検査しない。

準備中のレッスン（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)「準備中のレッスン」）は、次のように扱う。

- 「カリキュラム」のうち、`chapters.json` と [章とレッスンの一覧](../curriculum/chapters/index.md) の照合は、準備中のレッスンも対象にする。`lesson.json` の `title`・`kind`・`primary_elements` の照合と、ほかの項目は、準備中のレッスンを検査しない。
- 「要素マスタ」は、準備中のレッスンが 1 つでもある間は検査しない。
- 「要素と一覧」は、`lesson.json` を使わないので、準備中のレッスンがあっても検査する。

次の項目は警告とする。警告は結果に表示し、エラーにしない（CI を失敗にしない）。

| 警告の項目 | 内容 |
|---|---|
| 動画の長さ | 動画の長さ（[動画のタイムラインと合図](../content-format/video/timeline-and-cues.md)）が 1〜5 分の範囲にない |
| 文の長さ | `text` が 60 字を超える文がある |
| 読み | `say` に、読みの対応表（`content/readings.json`）の語がそのまま残っている |
