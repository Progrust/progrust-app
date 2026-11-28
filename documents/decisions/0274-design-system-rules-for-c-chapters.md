---
type: Decision
title: "ADR 0274"
description: "C の章のための見た目の決まり（エディタ・動画・回答の C の字句の色の当て方、コンソールと動画の clang の診断とメモリの検査の報告の色、clang の導入のコマンドを示す書き方、memory_error の表示、C の章の番号の印、レッスン一覧の「C 言語編」「Rust 編」の見出し）を、新しい色を作らずに既存のトークンと部品に当ててデザインシステムに足す"
---

- 日付: 2026-11-28
- 関連仕様: [画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステム」、[演習画面](../spec/screens/exercise-screen.md)、[動画でのコードの表示](../spec/content-format/video/code-display.md)、[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「画面に表示する内容」、[C のツール（clang）への依存](../spec/architecture/c-toolchain.md)「見つからない場合」、[レッスン一覧画面](../spec/screens/lesson-list.md)
- 置き換え: なし

## 背景

C の章を足したとき（[ADR 0262](0262-teach-c-chapters-before-rust.md)）、エディタ・動画・質疑応答の回答で C のコードを色分けし、動画では clang の診断とメモリの検査の報告も色付けすることにした（[ADR 0270](0270-highlight-c-code-in-editor-videos-and-chat.md)）。C の字句をどの色の区分に当てるかと、clang・メモリの検査の出力の色付けは、デザインシステムが決めるものとして未決事項に残し、実装計画を作る前に決めることにしていた。

あわせて、デザインシステム「Progrust」には、`clang` が見つからないときに導入のコマンド（`xcode-select --install`）を示す形と、判定結果のカードで `memory_error` を表示する形が無かった。部品は、`cargo`・`claude` のための、導入手順へのボタンを置く形だけを書いていた。

これらは、実装計画 M5（C 言語編）のタスクを作る前に、Claude が決めた。

## 決定

以下は、どれも Claude が決めた。値と書き方はデザインシステムの README に書いた。

- C の字句は、新しいトークンやクラスを作らず、Rust のコードと同じ `code-*` のトークン（動画では `tk-*` のクラス）に当てる（「Progrust」の README「コード」、「Progrust Video」の README「コードの色」）。
  - 理由: C のレッスンも Rust のレッスンも、同じ画面と同じ動画の部品で表示する。言語で色の意味が変わらないほうが、C の章から Rust の章へ進んだときに読み替えが要らない。
- プリプロセッサの指令（`#include` など）は、Rust の属性と同じ扱いにする。動画では `tk-a`（`code-macro`）にし、アプリ（エディタ・回答）では、Rust の属性と同じく色を付けない。ヘッダーの名前（`<stdio.h>`）は、どちらでも文字列の色にする。数値と `NULL` も、Rust の数値と同じく、動画では `tk-n`、アプリでは色なしにする。
  - 理由: どちらも「コンパイルの前に効く指示」で、Rust の属性と同じ区分に当てられる。アプリには属性と数値の区分が無い（「Progrust Video」の README「コードの色」の `tk-a`・`tk-n` はアプリ側にない区分）ので、アプリで C だけに色を付けると、Rust のコードと見え方が食い違う。
- コンソールの clang の診断は、clang が付ける ANSI の色を、いまの `ansi-*` のトークンへの割り当てのまま表示する（Apple clang 21.0.0 で確かめた色は、error が赤、warning がマゼンタ、note がシアン、`^` と修正の案が緑で、診断に引用されるコード片の字句にも色が付く）。rustc の区分に合わせて付け直さない（「Progrust」の Console の README）。
  - 理由: コンソールは ANSI の色を反映して表示する決まりで（[演習画面](../spec/screens/exercise-screen.md)）、clang の出力を解析して付け直す処理を持たずに済む。
- 動画の clang の診断とメモリの検査の報告は、rustc の出力と同じクラス（`out-error`・`out-warning`・`out-note`・`out-loc`・`out-caret`・`out-help`）で色付けする（「Progrust Video」の CompilerOutput の README）。動画では、clang の warning も rustc の warning と同じ色にする。
  - 理由: 動画は、色の意味をそろえて見せるほうが読みやすい。アプリのコンソールと違い、動画の出力は共通部品が解析して色を付けるので、意味の区分に当てられる。
- `clang` の導入のコマンドは、ボタンにせず、文の中に、押すと全体が選ばれる短い帯（`.pg-cmd`）で示す（「Progrust」の Banner・EnvCheck の README）。
  - 理由: アプリはコマンドを実行せず、URL も開かない（[C のツール（clang）への依存](../spec/architecture/c-toolchain.md)「見つからない場合」）。押せる形にすると、アプリが導入してくれると誤解されやすい。コピーのボタンは、新しい操作を足すことになるので置かない。
- `memory_error` は、判定結果のカードの面と見出しを、ほかの不正解と同じにし、誤りの種類の名前と行を、テストの一覧と、最初に失敗したテストケースの欄の見出しに出す。検査の報告は、色付けせずに等幅で出す（「Progrust」の JudgeResult の README）。
  - 理由: メモリの誤りも不正解の 1 つで、カードの色を分けると、ビルドの失敗（`warning-soft`）と並ぶ 3 つ目の状態に見える。報告は英語の文とアドレスを含み（[ADR 0272](0272-read-kind-and-location-from-sanitizer-reports.md)）、色付けの規則を足しても読みやすさはあまり変わらない。
- C の章の番号の印は、番号の前に「C」を付ける（`C1`）。レッスン一覧の「C 言語編」「Rust 編」の見出しは、章の色を持ち込まず、章の見出しより上の段の見出しにする（「Progrust」の ChapterBadge・LessonList の README）。
  - 理由: 番号が言語ごとに 1 から始まる（[ADR 0264](0264-number-chapters-per-language-and-c-chapter-ids.md)）ので、印でも言語を見分けられるようにする。印の大きさは、2 文字でも収まるので変えない。

## 検討した他の案

- 案: C のための字句の区分とトークン（プリプロセッサの色など）を新しく作る
  - 採用しなかった理由: Rust の属性と同じ意味の区分に当てられる。トークンを足すと、ライト・ダークの両方でコントラストを確かめ直し、「Progrust Video」の写しも合わせる手間が増える
- 案: コンソールでも、clang の診断を解析して、rustc の出力と同じ区分の色に付け直す
  - 採用しなかった理由: コンソールは ANSI の色を反映する決まりで、出力を解析する処理が新しく要る。warning の色がマゼンタになるだけで、意味は文言（`warning:`）で分かる
- 案: `memory_error` のカードを、新しい面の色にする
  - 採用しなかった理由: 上の「決定」の理由のとおり

## 影響

- 画面と動画で、clang の warning の色が違う（画面はマゼンタ、動画は注意の色）。
- 字句の分け方は、エディタ（Monaco の定義）と動画・回答（自作の走査）で別々に作る。両方を、デザインシステムの同じ表に合わせる。
- `.pg-cmd` は、`clang` 以外の導入のコマンドを示すときにも使える。
