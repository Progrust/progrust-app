---
type: Decision
title: "ADR 0264"
description: "C の章の ID を `c{NN}` にし、章の番号と章カラーを同じ言語の章の中での位置で決め、レッスン一覧を「C 言語編」「Rust 編」に区切る"
---

- 日付: 2026-11-28
- 関連仕様: [章とレッスンの並びの形式（chapters.json）](../spec/content-format/chapters-json.md)「章とレッスンの番号」、[章とレッスンの並びの形式（chapters.json）](../spec/content-format/chapters-json.md)、[章とレッスンの一覧の読み方](../spec/curriculum/chapters/how-to-read.md)、[レッスン一覧画面](../spec/screens/lesson-list.md)、[トップページ（ダッシュボード）](../spec/screens/top-page.md)、[動画のシーンの書き方](../spec/content-format/video/scenes.md)、[教材のコマンド](../spec/architecture/commands/content.md)、[学習記録・分析のコマンド](../spec/architecture/commands/learning-record.md)
- 置き換え: なし

## 背景

カリキュラムの最初に C の章を置くことにした（[ADR 0262](0262-teach-c-chapters-before-rust.md)）。章の番号と章カラーは、`chapters.json` での章の位置で決めていた（[ADR 0174](0174-number-chapters-and-lessons-by-position.md)）。C の章を前に置くと、Rust の章の番号が後ろにずれる（所有権は第4章から第7章になる）。章カラーも変わり、作成済みの動画で章の番号を読み上げている箇所は作り直しになる。

章 ID（`ch01`〜`ch23`）とレッスン ID は、作成済みの教材、要素マスタ、学習者の進捗、これまでの ADR と仕様の本文で使っている。

Claude は、ユーザーに次の 2 つの案を示した。

- C と Rust で別々に数える。C の章は C の中で 1 から数え、Rust の章はいまのまま第1〜23章にする。レッスン一覧は「C 言語編」「Rust 編」に区切る。
- 通し番号にする。C の章が先頭の番号になり、Rust の章は後ろにずれる。

Claude は、Rust の章の番号・章カラー・作成済みの動画のナレーションが変わらないことを理由に、別々に数える案を勧めた。ユーザーは、別々に数える案を選んだ。

## 決定

- 章の番号は、同じ言語の章の中での位置とする。[ADR 0174](0174-number-chapters-and-lessons-by-position.md) の「決定」の「章の番号は `chapters.json` での章の位置」を、この点で変える。レッスンの番号の決め方（章の番号と、章の中での位置）と、章カラーを章の番号で決めることは変えない（[章とレッスンの並びの形式（chapters.json）](../spec/content-format/chapters-json.md)「章とレッスンの番号」）。
  - 理由: ユーザーが選んだ。勧めた理由は、上の「背景」のとおり Claude が示したものである。
- レッスン一覧を「C 言語編」「Rust 編」に区切る（[レッスン一覧画面](../spec/screens/lesson-list.md)）。
  - 理由: ユーザーが選んだ案に含めて示した。番号が言語ごとに 1 から始まるので、区切りが無いと、同じ番号の章が 2 つ並んで見える。
- 次の細部は Claude が決めた。
  - C の章の ID を `c{NN}`、C のレッスンの ID を `c{NN}-{NN}`・`c{NN}-review` にする（[章とレッスンの並びの形式（chapters.json）](../spec/content-format/chapters-json.md)）。理由: Rust の章とレッスンの ID を変えずに済む。ID を見れば言語が分かる。
  - `chapters.json` の章に `language` を持たせ、省略したときは `rust` とする。C の章は、すべての Rust の章より前に並べる（同上）。理由: アプリは、判定のしかた、エディタの言語、番号の数え方を、章の言語で決める。省略時を `rust` にすれば、いまの `chapters.json` の Rust の章を書き換えずに済む。
  - 表示は、C の章を「C 第1章」、C のレッスンの番号を `C1-4` の形にする（同上）。理由: Rust の章とレッスンの番号と見分けられるようにする。
  - 章カラーは、C の章も章の番号で決める。C の第1章は、Rust の第1章と同じ色になる。理由: デザインシステムを変えずに済む。章カラーは、いまも第13章から同じ色を繰り返している（[ADR 0040](0040-twelve-chapter-colors.md)）。
  - トップページの 9 章ぶんの章ごとの完了率は、学習順での章の並びで選ぶ（[トップページ（ダッシュボード）](../spec/screens/top-page.md)）。理由: C の章から Rust の章へ進む途中でも、前後の章が続けて見える。
  - 章とレッスンを返すコマンドの戻り値に、言語と章の番号を持たせる（[教材のコマンド](../spec/architecture/commands/content.md)、[学習記録・分析のコマンド](../spec/architecture/commands/learning-record.md)）。理由: 番号の数え方をバックエンドの 1 か所に置き、画面ごとに数えさせない。

## 検討した他の案

- 案: 通し番号にする
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、Rust の章の番号と章カラーが変わり、作成済みの動画で章の番号を読み上げている箇所が作り直しになることである
- 案: C の章も `ch{NN}` の ID にし、Rust の章の ID を後ろにずらす
  - 採用しなかった理由: Claude の判断。作成済みの教材、要素マスタ、進捗、これまでの ADR と仕様の本文の ID が、すべて合わなくなる
- 案: `language` を、章ではなくレッスン（`lesson.json`）に持たせる
  - 採用しなかった理由: Claude の判断。1 つの章に 2 つの言語のレッスンを混ぜることは無い。準備中のレッスンには `lesson.json` が無く、言語が分からなくなる

## 影響

- Rust の章の ID・番号・章カラーは、いまと変わらない。
- 章の番号だけでは章が 1 つに決まらなくなった。章を指すときは、章 ID か、言語と章の番号の組を使う。
- デザインシステムの README の「第N章」の N は、これまでどおり章の番号である。
