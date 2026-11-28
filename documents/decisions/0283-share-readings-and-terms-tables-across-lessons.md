---
type: Decision
title: "ADR 0283"
description: "コードの読み方と教材の訳語を全レッスン共通の表でそろえ、決まった語の読みを読みの対応表（readings.json）に、組み合わせの読み方を仕様に、訳語を同梱する訳語の表（terms.json）に持つ"
---

- 日付: 2026-11-28
- 関連仕様: [ナレーションでのコードの読み方](../spec/content-guidelines/readings.md)、[教材で使う訳語](../spec/content-guidelines/terms.md)、[訳語の表の形式（terms.json）](../spec/content-format/terms-json.md)、[動画の作成ルール](../spec/content-guidelines/video.md)「台本」、[教材のディレクトリ構成](../spec/content-format/directory-structure.md)、[教材チェックツール](../spec/content-guidelines/check-tool.md)、[動画の音声](../spec/content-format/video/voice.md)
- 置き換え: なし

## 背景

教材は、レッスンごとに Claude が作る予定である。ユーザーは、次の 2 つの表を用意することにした。

- 読み方の表: `&s`、`::`、`->`、`mut`、`Vec<T>` などを音声でどう読むかを決めておく。レッスンによって読み方が変わるのを防ぐためである。
- 訳語の表: 「借用」と「参照」、「ベクタ」と「ベクター」のような言葉のゆれをそろえる。

これまで、読み間違えやすい語の読みは、読みの対応表（`content/readings.json`）に、語と読みの組で持っていた（[ADR 0065](0065-separate-text-and-say-in-script.md)）。`say` にこの表の語がそのまま残っていると、教材チェックツールが警告する。

表の中身は Claude が案を作り、ユーザーが確かめた。ユーザーは、`char` を「キャラ」、`Vec` を「ベクタ」と読むことにし、ほかは案のとおりとした。

## 決定

- 決まった語（キーワード、型名、関数名、マクロ名、ツールの名前）の読みは、読みの対応表（`content/readings.json`）に足す。記号と、型や変数の組み合わせの読み方は、仕様の [ナレーションでのコードの読み方](../spec/content-guidelines/readings.md) に書く。以下の理由は Claude が書いた。
  - 理由: 読みの対応表に足せば、教材チェックツールの警告が効く。`&s` や `Vec<i32>` は変数名や型によって形が変わるので、語と読みの組では書けない。
- 訳語は、訳語の表（`content/terms.json`）に持ち、アプリに同梱する（[訳語の表の形式（terms.json）](../spec/content-format/terms-json.md)）。
  - 理由: 質疑応答の AI にも渡すことにしたので（[ADR 0284](0284-pass-terms-table-to-ai.md)）、アプリが読めるところに置く必要がある。仕様の文書は同梱しない。
- 教材チェックツールで、訳語の表の形式を検証する（[教材チェックツール](../spec/content-guidelines/check-tool.md)）。
  - 理由: アプリが読んで AI に渡すデータなので、形式が崩れたまま同梱されないようにする。
- 読みには「、」を入れない。
  - 理由: 音声を作るツールは、`say` の句読点の数と、音声の中の間の数を合わせる（[ADR 0198](0198-find-comma-pauses-by-matching-silences-to-punctuation.md)）。
- 表にない訳語は TRPL の日本語版に合わせ、カタカナ語の末尾の長音を付けない。
  - 理由: 仕様と教材は、すでにこの書き方（コンパイラ、ベクタ、ポインタ）である。学習者が TRPL の日本語版を並べて読んでも、語が食い違わない。

## 検討した他の案

- 案: 読み方の表をすべて仕様の文書に書き、読みの対応表には足さない
  - 採用しなかった理由: 教材チェックツールの警告が効かない
- 案: 訳語の表を仕様の文書（Markdown の表）に書く
  - 採用しなかった理由: 仕様の文書はアプリに同梱しないので、AI に渡せない
- 案: 訳語の表を指示文（`prompts/`）に書き写す
  - 採用しなかった理由: 指示文ごとに写しができ、表を直したときに食い違う

## 影響

- 読みの案は、VOICEPEAK で鳴らして確かめていない。音声を作ったときに読み間違えたら、表を直す（[教材レビューのチェックリスト](../spec/content-guidelines/review-checklist.md)）。
- 訳語の表を読む処理と、形式の検証は、まだ実装していない。
