---
type: Decision
title: "ADR 0280"
description: "`&T` を「共有参照」、`&mut T` を「可変参照」と呼び、「不変」を変数にだけ使い、要素 `borrow.immutable` の ID を変えずに名前を「共有参照」にする"
---

- 日付: 2026-11-28
- 関連仕様: [教材で使う訳語](../spec/content-guidelines/terms.md)「参照の呼び方」、[ch05 参照と借用](../spec/curriculum/chapters/ch05-references-and-borrowing.md)
- 置き換え: なし

## 背景

カリキュラムと要素マスタでは、`&T` を「不変の参照」、`&mut T` を「可変の参照」と呼んでいた（ch05-01・ch05-02 のタイトル、要素 `borrow.immutable`・`borrow.mutable` の名前）。

ユーザーは、`&T` を「共有参照」、`&mut T` を「可変参照」と呼ぶことにした。ユーザーが示した理由は次のとおりである。

- `&T` を「不変」と呼ぶと、`Cell` や `AtomicUsize` のように、共有参照なのに値を変更できる型と矛盾する。
- 一部の企業やコミュニティ（Google など）では、「共有参照」という呼び方がすでに主流である。
- `&mut T` は、「排他参照」より「可変参照」のほうが理解しやすい。`mut` というキーワードがすでにあるからである。「可変 = 排他」は Rust の共通理解でもある。

## 決定

- 教材では、`&T` を「共有参照」、`&mut T` を「可変参照」と呼ぶ（[教材で使う訳語](../spec/content-guidelines/terms.md)「参照の呼び方」）。ch05-01・ch05-02 のタイトルと、要素 `borrow.immutable`・`borrow.mutable` の名前と説明を、この呼び方にする（[ch05 参照と借用](../spec/curriculum/chapters/ch05-references-and-borrowing.md)）。
- 要素 ID `borrow.immutable` は変えない。以下の理由は Claude が書いた。
  - 理由: 要素 ID は、一度公開したら変えない決まりで（[要素マスタの形式（elements.json）](../spec/content-format/elements-json.md)）、学習記録と分析ファイルが ID で要素を指す。学習者に見えるのは名前（`name`）だけである。
- 「不変」「可変」は、変数（`let` と `let mut`）にも使う。Claude が提案し、ユーザーが承認した。
  - 理由: コンパイラの出力が、変数を immutable variable と呼ぶ。参照の呼び方を変えても、変数の呼び方はコンパイラの出力とそろえたほうが、学習者がエラーを読みやすい。
- コンパイラの出力の immutable borrow・mutable borrow は、英語のまま載せ、共有参照・可変参照として借用していることだと言葉で説明する。

## 検討した他の案

- 案: `&T` を「不変参照」と呼ぶ（これまでの呼び方）
  - 採用しなかった理由: 共有参照を通して値を変えられる型（`Cell`・`AtomicUsize`）と矛盾する
- 案: `&mut T` を「排他参照」と呼ぶ
  - 採用しなかった理由: `mut` というキーワードがあるので、「可変参照」のほうが理解しやすい。「可変 = 排他」は Rust の共通理解でもある
- 案: 要素 ID も `borrow.shared` に変える
  - 採用しなかった理由: 要素 ID は変えない決まりである。学習者に ID は見えない

## 影響

- コンパイラの出力（immutable borrow）と教材の語が違うので、エラーを説明するところで、同じものを指すことを伝える必要がある。
- TRPL の日本語版など、ほかの資料では「不変参照」と書かれていることがある。必要なところで「不変参照と呼ばれることもあります」と添える（[教材で使う訳語](../spec/content-guidelines/terms.md)「訳語の表」）。
