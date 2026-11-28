---
type: Decision
title: "ADR 0269"
description: "C の章の教材は、説明を短くまとめず、既習の C の内容だけを使い、Rust のコードを c01-01 の動画以外に登場させず、演習の出力を実行のたびに同じになるものにし、模範解答をメモリの検査で誤りが出ないものにし、c01-02〜c01-04 の演習はテストケースを 1 件以上でよいとする"
---

- 日付: 2026-11-28
- 関連仕様: [C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)、[未登場の要素を使わない](../spec/content-guidelines/no-unseen-elements.md)、[演習の作成ルール](../spec/content-guidelines/exercise.md)、[教材チェックツール](../spec/content-guidelines/check-tool.md)、[教材レビューのチェックリスト](../spec/content-guidelines/review-checklist.md)、[演習問題生成で AI に渡す入力](../spec/exercise-generation/ai-input.md)
- 置き換え: なし

## 背景

カリキュラムの最初に C の章を置いた（[ADR 0262](0262-teach-c-chapters-before-rust.md)）。教材は AI も作るので、C の章の教材の書き方を、決まりとして定めておく必要があった。C のレッスンは要素を持たない（[ADR 0265](0265-c-lessons-without-elements-records-and-generation.md)）ので、未登場の要素を使わない決まりと、教材チェックツールの既習の要素の検査は、C のレッスンにはそのまま当てはまらない。

ユーザーは、C の量について次のように示している。「C の量は、演習問題でメモリの動きを色々と試す際に困らない程度の基本は抑えておきたいです。それが5〜8 レッスンで習得できればよいですが、難しいなら広げてもよいです。重要なのはレッスンの数ではなく、Cのメモリを学ぶうえで十分な知識が学べるかどうかです。このアプリはRust学習のものだからCは短く作ったほうがいいという考えで進めないでください。演習問題は難しめに作るという方針もそうですが、つまずきながら学ぶことで難しいRustという言語を深く理解することがこのアプリの目的です。」

ユーザーは、最初のレッスンで、最初に C 言語を学ぶ理由を、Rust の特徴も含めて説明するとも示している（[ADR 0262](0262-teach-c-chapters-before-rust.md) の「背景」）。

## 決定

- Rust の学習アプリであることを理由に、C の説明を短くまとめない（[C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)「目的と説明の深さ」）。
  - 理由: 上の「背景」のユーザーの発言のとおり。
- 次の細部は Claude が決めた。
  - C のレッスンには、そのレッスンと、それより前の C のレッスンの「主に扱う内容」にある C の構文・関数・型だけを登場させる。教材チェックツールでは検証せず、レビューで確かめる（[C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)「既習の内容だけを使う」）。理由: Rust のレッスンの、未登場の要素を使わない決まりと同じ考え方である。C のレッスンには要素が無いので、範囲は「主に扱う内容」で決める。
  - C のレッスンには、Rust のコードを登場させない。c01-01 の動画だけは、Rust のコンパイラが誤りを指摘する様子を見せるために、載せてよい（[C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)「Rust に触れるとき」）。理由: Rust の基本と C の基本が混ざらないようにする（[ADR 0262](0262-teach-c-chapters-before-rust.md) の、位置についてのユーザーの理由）。c01-01 は、Rust の特徴を説明するレッスンなので、例外にする。
  - コンパイラの出力、実行結果、メモリの検査の報告には、実際に得た出力を使う。メモリの誤りは、検査なしで実行した結果と、検査の報告の両方を見せる（[C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)「動画」）。理由: Rust のレッスンのコンパイラの出力と同じ考え方である。両方を見せることで、黙って動いてしまうことと、実際には誤りであることが伝わる。
  - 演習の出力は、実行のたびに同じになるものにする。アドレスの値、初期化していない領域の値、未定義動作の結果を出力させない（[C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)「演習」）。理由: 判定は標準出力の比較で行う（[ADR 0266](0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)）。
  - 模範解答は、メモリの検査で誤りが報告されないものにし、確保した領域をすべて解放する（同上、[教材チェックツール](../spec/content-guidelines/check-tool.md)）。理由: 提出はメモリリークも誤りとして判定する（[ADR 0266](0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)）。
  - c01-02〜c01-04 の演習は、入力のない、出力が固定の演習にし、テストケースを 1 件以上でよいとする。[ADR 0146](0146-one-test-case-allowed-in-ch01-and-ch02.md) の「決定」の「それ以外のレッスンは、これまでどおり 3 件以上とする」を、この 3 つのレッスンについて変える（[C の章の教材の作成ルール](../spec/content-guidelines/c-lessons.md)「演習」）。理由: 標準入力を c01-05 で学ぶまでは、入力を変えたテストケースを作れない。[ADR 0146](0146-one-test-case-allowed-in-ch01-and-ch02.md) が ch01・ch02 を例外にした理由と同じである。
  - 演習問題生成の AI には、C の章の教材の決まりを渡さない（[演習問題生成で AI に渡す入力](../spec/exercise-generation/ai-input.md)）。理由: 生成するのは Rust の問題だけである（[ADR 0265](0265-c-lessons-without-elements-records-and-generation.md)）。

## 検討した他の案

- 案: C のレッスンにも、Rust のコードを対比として載せる
  - 採用しなかった理由: 学習者はまだ Rust を学んでいない。Rust の基本と C の基本が混ざる。対比は、Rust のレッスンの側で行う（[ADR 0268](0268-contrast-rust-lessons-with-c-chapters.md)）
- 案: 既習の C の内容を、教材チェックツールで検証できるようにする（C の内容を要素にする）
  - 採用しなかった理由: C のレッスンには要素を持たせない（[ADR 0265](0265-c-lessons-without-elements-records-and-generation.md)）。Rust のレッスンでも、コード中の構文が未登場でないことは、レビューで確かめている

## 影響

- C のレッスンで既習でない内容を使っていないことは、機械では確かめられない。
- 教材チェックツールは、C の演習のテストケースの件数を、レッスン ID で決める（c01-02〜c01-04 は 1 件以上）。
