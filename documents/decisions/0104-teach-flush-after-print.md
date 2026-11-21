---
type: Decision
title: "ADR 0104"
description: "print! で出したプロンプトを入力の前に表示するための flush を、ch09-05 で教える"
---

- 日付: 2026-11-21
- 関連仕様: [ch09 エラー処理](../spec/curriculum/chapters/ch09-error-handling.md)、[演習の作成ルール](../spec/content-guidelines/exercise.md)「問題」
- 置き換え: なし

## 背景

Issue #16 の技術検証で、`print!("名前を入力: ")` の後に `flush()` しないと、プロンプトが入力の前に表示されないことを確かめた（[学習者のプログラムの子プロセスとしての扱いの技術検証](../verification/issue-16-learner-process.md)）。Rust の標準出力は行単位でバッファされ、`stdin().read_line` は標準出力を flush しない。端末で実行しても同じで、アプリの側では直せない。

この書き方が要るのは、改行なしで出力してから入力を待つときである。`print!` は ch01-05 で、標準入力の読み方は ch09-05 で学ぶ（[ch01 Rust の基本](../spec/curriculum/chapters/ch01-rust-basics.md)、[ch09 エラー処理](../spec/curriculum/chapters/ch09-error-handling.md)）。

Claude は、この書き方が初めて出るレッスンで、すぐに表示するための 1 行を教えることを推奨した。ユーザーは、この推奨のとおりでよいと示した（「Bも推奨通りで大丈夫です。」）。教えるレッスンを ch09-05 にしたことと、以下の理由は Claude が書いた。

## 決定

- ch09-05（標準入力を読む）で、`print!` で出したプロンプトを入力の前に表示する `flush` を教える（[ch09 エラー処理](../spec/curriculum/chapters/ch09-error-handling.md)）。
  - 理由: 標準入力の読み方は ch09-05 で学ぶ。それより前の ch03〜ch08 では、入力の読み込みをおまじないとして初期コードに用意するだけで、学習者がプロンプトを書く場面はない（[レッスンの演習を作るときの注意](../spec/curriculum/exercise-constraints.md)）。`print!` を学ぶ ch01-05 では入力を待たないので、プログラムの終わりにまとめて表示されても困らない。
- 教材のコードで、改行なしで出力した後に入力を待つときは、出力の後に `flush` する（[演習の作成ルール](../spec/content-guidelines/exercise.md)「問題」）。
  - 理由: 教材のコードを実行してプロンプトが出ないと、学習者がアプリの不具合と取り違える。

## 検討した他の案

- 案: 教えずに、教材のコードで `print!` のプロンプトを使わない
  - 採用しなかった理由: 学習者が自分でプロンプトを書いたときに、表示されない理由が分からない
- 案: アプリの側で、学習者のプログラムの出力をすぐに表示させる
  - 採用しなかった理由: 出力は学習者のプログラムの中でたまっていて、アプリからは取り出せない

## 影響

- `flush` を呼ぶには `use std::io::Write;` が要る。`use` とトレイトはまだ学んでいないので、ch09-05 では、この 1 行をおまじないとして示す（[未登場の要素を使わない](../spec/content-guidelines/no-unseen-elements.md)）。
