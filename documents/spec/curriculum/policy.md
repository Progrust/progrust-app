---
type: Specification
title: "カリキュラムの方針"
description: "Rust 固有でない内容はまとめて扱い Rust 固有の内容は細かく分けること、The Rust Programming Language（TRPL）を網羅の目安とし文章・コード・題材は使わないこと、Rust を理解するうえで必要な周辺知識を扱うこととその置き方、主要素の数と章末復習レッスンの置き方の参照先を定める。"
---

- 1 レッスンの主要素の数と章末復習レッスンの置き方は [教材作成におけるカリキュラムの決まり](../content-guidelines/curriculum.md) に従う。
- 想定する学習者（[想定する学習者](../content-guidelines/target-learner.md)）に合わせ、Rust 固有でない内容（`while`、イテレータの汎用的なアダプタなど）はまとめて扱い、Rust 固有の内容（所有権・借用・パターン・ライフタイムなど）は細かく分ける。
- The Rust Programming Language（以下 TRPL）を、扱う内容の網羅の目安とする。章立て・順序・題材は独自に組み立て、TRPL の文章・コード・題材のプログラム（数当てゲーム、minigrep、Web サーバーなど）は使わない。
- Rust を理解するうえで必要な周辺知識（[想定する学習者](../content-guidelines/target-learner.md)）は、TRPL が扱っていないものも扱う。
- 周辺知識のうち、特定の Rust の要素の理由になるものは、その要素のレッスンの「主に扱う内容」に含める。複数のレッスンの前提になるものは、動画のみのレッスン（[教材作成におけるカリキュラムの決まり](../content-guidelines/curriculum.md)）にして、それを前提とするレッスンより前に置く。
