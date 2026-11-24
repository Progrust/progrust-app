---
type: Glossary
title: "教材に関する用語"
description: "章、レッスン、動画、台本、演習、要素、判定方式など、教材に関する用語の英語表記と定義を定める。"
---

| 用語 | 英語表記（コード上の名前） | 定義 |
|---|---|---|
| 教材 | content | アプリに同梱する章・レッスン・要素マスタの総称。形式は [教材データ形式](../content-format/index.md) |
| 章 | chapter | レッスンをまとめる単位。ID は `ch{NN}`（例: `ch04`） |
| レッスン | lesson | 「動画 1 本 + 演習 1 問」の学習単位。動画のみのレッスンは演習を持たない。ID は `ch{NN}-{NN}`（例: `ch04-03`） |
| 章末復習レッスン | chapter review | 各章の最後に置く、演習のみのレッスン。ID は `ch{NN}-review` |
| 動画のみのレッスン | video-only lesson | 周辺知識だけを扱う、演習のないレッスン（`lesson.json` の `kind` が `video_only`）。ID はレッスンと同じ `ch{NN}-{NN}` |
| 準備中のレッスン | lesson in preparation | `chapters.json` にあるが、レッスンのディレクトリがまだ無いレッスン（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)「準備中のレッスン」） |
| 周辺知識 | background knowledge | Rust の文法や機能そのものではないが、Rust を理解するうえで必要な知識（メモリの仕組み、スレッドとデータ競合など。[想定する学習者](../content-guidelines/target-learner.md)） |
| 動画 | video | レッスンの知識をインプットするための、ナレーションつきの 16:9 の教材。動画ファイルではなく、アプリの中で描画する（[レッスンの動画の形式（video/）](../content-format/video/index.md)） |
| シーン | scene | 動画を構成する場面の単位（表紙、まとめ など） |
| 台本 | script | 動画のナレーションを文ごとに並べたデータ（`script.json`。[動画の台本（script.json）](../content-format/video/script-json.md)） |
| 文 | sentence | 台本の 1 件。ID を持ち、音声 1 ファイルに対応する |
| ナレーション | narration | 台本を読み上げた音声 |
| 合図 | cue | 絵を動かす位置の指定。「文の開始 + 秒」で表す（[動画のタイムラインと合図](../content-format/video/timeline-and-cues.md)） |
| 声の設定 | voice settings | 音声を作るときの VOICEPEAK のナレーターと設定値。全レッスン共通（`content/voice.json`。[動画の音声](../content-format/video/voice.md)） |
| 読みの対応表 | readings | 読み間違えやすい語と、`say` に書く読みの対応。全レッスン共通（`content/readings.json`。[動画の音声](../content-format/video/voice.md)） |
| 再生中の文 | current sentence | 再生位置にあたる文（決め方は [動画画面](../screens/video-screen.md)） |
| 文の頭 | sentence start | 文の開始位置（[動画画面](../screens/video-screen.md)） |
| 検索ワード | tags | レッスン検索の対象にする語（`lesson.json` の `tags`） |
| 演習 | exercise | コードを書いて提出し、正誤判定を受ける問題。レッスンの演習と生成問題がある |
| 生成問題 | generated exercise | AI が生成した演習。ID は `gen-{生成 ID}` |
| 要素 | element | Rust の文法・概念の単位（例: 所有権のムーブ）。要素 ID（例: `ownership.move`）で参照する |
| 要素マスタ | element master | 全要素の定義（`elements.json`） |
| 分類 | category | 要素 ID の `.` より前の部分。Rust の機能のまとまりを表す（[要素の分け方と要素 ID の付け方](../content-guidelines/elements.md)「要素 ID」） |
| 主要素 | primary elements | そのレッスン・演習の学習対象の要素 |
| 副要素 | secondary elements | 学習対象ではないが演習に登場する既習の要素 |
| 学習順 | learning order | 全レッスンを章・レッスンの順に一列に並べた順序（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)） |
| 章の番号 | chapter number | `chapters.json` での章の位置（1 から）。画面と動画に出す章の番号で、章カラーもこれで決まる（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)「章とレッスンの番号」） |
| レッスンの番号 | lesson number | 「{章の番号}-{章の中での位置}」の形の番号（例: `4-4`）。画面と動画に出す（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)「章とレッスンの番号」） |
| 導入レッスン | introducing lesson | 学習順で最初にその要素を主要素とするレッスン（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)） |
| 既習の要素 | learned elements | 導入レッスンが、あるレッスンより学習順で前にある要素（[章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md)） |
| 問題文 | problem | 演習の課題を説明する文章 |
| 初期コード | starter code | 演習を開いたときにエディタに表示されるコード |
| 模範解答 | solution | 判定を通過する作者の解答コード |
| 解説 | explanation | 正解後に表示する、考え方と模範解答の説明 |
| 判定方式 | judge mode | `stdout`（標準出力の比較）または `test`（隠しテスト） |
| テストケース | case | 判定の 1 単位。`stdout` 方式では入力と期待出力の組、`test` 方式では 1 つのテスト関数。学習者のテストの判定は、まとめて 1 件とする |
| 学習者のテスト | learner tests | 学習者が自分で書いたテスト（単体テスト、`tests/` の統合テスト、ドキュメントテスト）。テスト実行で動かせ、`min_learner_tests` のある演習では判定に含める（[コードの提出（submit）](../exercise-runner/submit.md)「学習者のテストの判定」） |
| 許可クレート | allowed crates | 演習で使ってよい外部クレートの一覧。画面の文言では「外部クレート」と呼ぶ |
| 初回準備の状態 | preparation status | バックエンドが持つ、許可クレートの初回準備のいまの状態（`not_started`・`preparing`・`ready`・`failed`。[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「初回準備の状態」） |
