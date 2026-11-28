---
type: Decision
title: "ADR 0262"
description: "カリキュラムの最初に C 言語の章を置き、メモリの仕組みを演習で体験させてから Rust を教え、最初のレッスンを C から学ぶ理由を説明する動画のみのレッスンにする"
---

- 日付: 2026-11-28
- 関連仕様: [カリキュラムの方針](../spec/curriculum/policy.md)「C の章と Rust の章」、[想定する学習者](../spec/content-guidelines/target-learner.md)、[章とレッスンの一覧](../spec/curriculum/chapters/index.md)、[c01 C の基本](../spec/curriculum/chapters/c01-c-basics.md)、[教材作成におけるカリキュラムの決まり](../spec/content-guidelines/curriculum.md)、[レッスンの受講順](../spec/lesson-flow/order.md)
- 置き換え: なし

## 背景

想定する学習者は他のプログラミング言語の経験者で、Rust の文法と Rust 固有の概念に集中するとしていた（[ADR 0002](0002-target-experienced-programmers.md)）。ガベージコレクションのある言語では意識しなくてよい周辺知識は既知とせず、メモリの仕組みは、演習のない動画のみのレッスン ch04-01 で説明することにしていた（[ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md)、[ADR 0119](0119-video-only-lessons-for-background-knowledge.md)）。

ユーザーは、実装の途中で、次の案を示した。「Rustのコンパイラーは危険なコードを教えてくれるが、C言語未経験者はその警告がなぜ危険なのか理解できない。それを理解しながらRustを学べるようにしたい。一番最初はRustではなく C言語 の基本を学ぶ。（C言語が使えるようになるために学ぶのではなく、Rustを深く理解するためにC言語を学ぶ）そして C言語 の難しさを演習問題で体感してもらう。そこで身についたメモリ管理の複雑さを以下にRustが克服できるかを見せる。」

Claude は、学習の面と、このアプリで実現できるかを調べて、次のように答えた。

- 学習の面では意味が大きい。動画のみにした理由は「概念の説明は演習問題にしにくい」だった（[ADR 0119](0119-video-only-lessons-for-background-knowledge.md)）が、C で書かせれば演習にできる。
- Rust を動かせる Mac には Apple の Command Line Tools が入っていて、C コンパイラの clang が含まれるので、追加の導入は要らない。メモリの誤りは、clang の検査の機能で見つけられる（確かめた内容は [ADR 0266](0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)）。
- C の章の位置は、所有権の章（ch04）の直前を勧める。C の量は、メモリに絞った 5〜8 レッスンを勧める。

ユーザーは、位置について次のように答えた。「C の章の位置は一番最初にしましょう。Rust → C → Rustのようになってしまうと、逆に最初に学んだRustの基本とCの基本が混ざって混乱してしまうと思います。一番最初は動画教材のみとして、「なぜ最初にC言語を学ぶのか」を、Rustの特徴も含めて最初に説明しておきます。」

## 決定

- カリキュラムの最初に C の章を置き、その後に Rust の章を置く（[カリキュラムの方針](../spec/curriculum/policy.md)「C の章と Rust の章」）。
  - 理由: 上の「背景」の、ユーザーが最初に示した案のとおり。
- C の章の位置は、一番最初にする。
  - 理由: 上の「背景」の、位置についてのユーザーの発言のとおり。
- 最初のレッスン c01-01 は動画のみのレッスンにし、最初に C 言語を学ぶ理由を、Rust の特徴も含めて説明する（[c01 C の基本](../spec/curriculum/chapters/c01-c-basics.md)）。
  - 理由: 上の「背景」の、位置についてのユーザーの発言のとおり。
- [ADR 0002](0002-target-experienced-programmers.md) の「決定」の「Rust の文法と Rust 固有の概念に集中する」を、C の章では C の文法とメモリの扱いに、Rust の章では Rust の文法と Rust 固有の概念に集中する、に変える（[想定する学習者](../spec/content-guidelines/target-learner.md)）。一般的なプログラミングの概念を既知とすることは変えない。
- [ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md) の「決定」の「複数のレッスンの前提になるものは動画のみのレッスンにする」を、メモリの仕組みについては、C の章で演習のあるレッスンとして扱う、に変える。スレッドとデータ競合（ch20-01）は、動画のみのレッスンのままにする（[カリキュラムの方針](../spec/curriculum/policy.md)「Rust の章」）。
- 次の細部は Claude が決めた。
  - 動画のみのレッスンが扱ってよい内容に、学習の進め方の説明（c01-01）を足す（[教材作成におけるカリキュラムの決まり](../spec/content-guidelines/curriculum.md)）。理由: c01-01 は、周辺知識の説明ではなく、これから何をどう学ぶかの説明である。動画のみのレッスンにできるのを、章とレッスンの一覧で定めたレッスンだけとする決まり（[ADR 0119](0119-video-only-lessons-for-background-knowledge.md)）は変えない。
  - C の章を飛ばす仕組みは設けない。推奨レッスンの決め方も変えない（[レッスンの受講順](../spec/lesson-flow/order.md)）。理由: 受講順は制限していない（[ADR 0018](0018-no-lesson-order-lock.md)）ので、Rust のレッスンを直接開ける。想定する学習者は、C の経験を前提にしない。

## 検討した他の案

- 案: C の章を、所有権の章（ch04）の直前に置く（Claude の最初の提案）
  - 採用しなかった理由: 上の「背景」の、位置についてのユーザーの発言のとおり。Rust の基本の後に C の基本を学ぶと、2 つが混ざって混乱する
- 案: これまでどおり、メモリの仕組みを動画だけで説明する（[ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md)、[ADR 0119](0119-video-only-lessons-for-background-knowledge.md)）
  - 採用しなかった理由: 誤りを自分の手で体験できない。上の「背景」の、ユーザーが最初に示した案は、C の難しさを演習で体感させることを求めている
- 案: C を使わず、Rust の `unsafe` で同じ誤りを体験させる
  - 採用しなかった理由: Claude の判断。`unsafe` と生ポインタは ch23 で扱う内容で、所有権の前に持ってくるには Rust の文法が足りない。Rust の基本と混ざらないようにするという、ユーザーの位置についての理由にも合わない

## 影響

- Rust の最初のレッスンにたどり着くまでに、C の章の 30 レッスン（[ADR 0263](0263-c-chapters-scope-three-chapters.md)）がある。[ADR 0002](0002-target-experienced-programmers.md) が前提知識のレッスンを足さなかった理由（Rust 固有の概念にたどり着くまでが長くなる）は、ここで受け入れた。
- [ADR 0002](0002-target-experienced-programmers.md) は置き換えない。想定する学習者は、他のプログラミング言語の経験者のままである。
- C の演習を動かす仕組み（[ADR 0266](0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)）、章の番号（[ADR 0264](0264-number-chapters-per-language-and-c-chapter-ids.md)）、学習記録などの扱い（[ADR 0265](0265-c-lessons-without-elements-records-and-generation.md)）、質疑応答（[ADR 0267](0267-pass-c-lesson-titles-and-summaries-to-chat.md)）、Rust のレッスンの書き直し（[ADR 0268](0268-contrast-rust-lessons-with-c-chapters.md)）、教材の決まり（[ADR 0269](0269-c-lesson-content-rules.md)）、コードの色分け（[ADR 0270](0270-highlight-c-code-in-editor-videos-and-chat.md)）を、あわせて決めた。
- アプリ・教材チェックツール・デザインシステム・教材は、この ADR の時点では変えていない。ユーザーは次のように示した。「実装側の修正は実装タスクのIssueとして別途対応するので、実装側の修正は今はやらなくて大丈夫です。実装の修正は次のマイルストーンにしてやりましょう。仕様書やドキュメントの修正などをお願いします。」
- すでに作ってある ch04-01 の教材は、内容が変わる（[ADR 0268](0268-contrast-rust-lessons-with-c-chapters.md)）ので、作り直しが要る。
- 推奨レッスンは、学習順で最初の未完了のレッスンである。C の章が教材に入ると、Rust のレッスンを進めていた学習者も、次にやるべきレッスンが c01-01 になる。
