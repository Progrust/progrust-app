---
type: Decision
title: "ADR 0270"
description: "エディタ・動画・質疑応答の回答で C のコードも色分けし、動画では clang の診断とメモリの検査の報告も色付けし、字句の区分と色をデザインシステムに足すことは未決事項に残す"
---

- 日付: 2026-11-28
- 関連仕様: [動画でのコードの表示](../spec/content-format/video/code-display.md)、[演習画面](../spec/screens/exercise-screen.md)、[質疑応答の画面](../spec/qa-chat/screen.md)、[アプリの技術構成](../spec/architecture/tech-stack.md)、[教材データ形式の未決事項](../spec/content-format/open-questions.md)、[画面仕様の未決事項](../spec/screens/open-questions.md)
- 置き換え: なし

## 背景

カリキュラムの最初に C の章を置いた（[ADR 0262](0262-teach-c-chapters-before-rust.md)）。コードの色分けは、Rust だけを対象にしていた。

- 動画のコードは、Rust の字句を分ける自作の走査で色分けし、色付けするのは `rust` だけとしていた。コンパイラの出力の色付けも、rustc の出力の形だけを扱っていた（[ADR 0188](0188-write-rust-highlighter-by-hand.md)）。
- エディタは、Monaco Editor から全言語の定義を除き、Rust だけを自分で登録していた（[ADR 0225](0225-lean-monaco-entry-rust-tokens-and-theme-from-css-variables.md)）。
- 動画のデザインシステム「Progrust Video」も、色分けするのは Rust だけ、色付けする出力は rustc だけ、と定めている。

色と見た目は、デザインシステムを唯一の定義にしている（[ADR 0044](0044-design-system-as-source-of-visuals.md)）。ユーザーは、実装の側の変更は後のマイルストーンで行うと示している（[ADR 0262](0262-teach-c-chapters-before-rust.md) の「影響」）。

## 決定

以下は、どれも Claude が決めた。

- 動画のコードブロックは、`language` が `c` のコードも色分けする。`text` のコードブロックでは、clang の診断とメモリの検査の報告も色付けする。[ADR 0188](0188-write-rust-highlighter-by-hand.md) の「決定」の「色付けするのは `rust` だけ」と、色付けする出力を rustc の出力だけとしている点を、この点で変える（[動画でのコードの表示](../spec/content-format/video/code-display.md)）。
  - 理由: C の章の動画は、C のコード、clang の出力、メモリの検査の報告を中心に見せる。色分けが無いと、Rust のレッスンより読みにくくなる。
- エディタは、C の演習では C の構文を色分けする。[ADR 0225](0225-lean-monaco-entry-rust-tokens-and-theme-from-css-variables.md) の「決定」の理由の「言語は Rust だけを使い」を、この点で変える（[演習画面](../spec/screens/exercise-screen.md)、[アプリの技術構成](../spec/architecture/tech-stack.md)）。
  - 理由: 上と同じ。
- 質疑応答の回答の中の C のコードブロックも色分けする（[質疑応答の画面](../spec/qa-chat/screen.md)）。
- C の字句をどの色の区分に当てるか（プリプロセッサの行など）と、clang・メモリの検査の出力の色付けを、デザインシステムに足すことは、未決事項に残す（[教材データ形式の未決事項](../spec/content-format/open-questions.md)、[画面仕様の未決事項](../spec/screens/open-questions.md)）。
  - 理由: デザインシステムのファイルは、今回は変えない。区分と色はデザインシステムが決めるもので、仕様では決めない。実装計画を作る前に決める。
- 色分けの作り方（自作の走査に C を足すか、Monaco の C の定義を使うか など）は、ここでは決めない。実装するときに決める。

## 検討した他の案

- 案: C のコードは色分けしない（ほかの言語と同じ扱いのままにする）
  - 採用しなかった理由: C のレッスンは 30 あり、コードを読む量が多い。色分けの無いコードは、型・キーワード・文字列の見分けがつきにくい

## 影響

- デザインシステム「Progrust」「Progrust Video」と、仕様が、実装されるまで食い違う（デザインシステムは Rust だけを色分けすると書いている）。未決事項を決めるときに、デザインシステムを先に直す。
- [ADR 0188](0188-write-rust-highlighter-by-hand.md) と [ADR 0225](0225-lean-monaco-entry-rust-tokens-and-theme-from-css-variables.md) の、Rust の色分けの作り方は変えない。
