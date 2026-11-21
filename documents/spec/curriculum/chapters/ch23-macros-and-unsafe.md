---
type: Specification
title: "ch23 マクロと unsafe"
description: "ch23 マクロと unsafe の章の説明と、宣言的マクロの基本、マクロの繰り返し、unsafe と生ポインタ、unsafe を安全な API で包む方法を扱うレッスン（ch23-01〜ch23-04 と章末復習）の ID・タイトル・主に扱う内容を定める。"
---

宣言的マクロでコードを生成する方法と、unsafe Rust を学ぶ

| ID | タイトル | 主に扱う内容 |
| --- | --- | --- |
| ch23-01 | 宣言的マクロの基本 | `macro_rules!`、マッチャーとフラグメント指定子 |
| ch23-02 | マクロの繰り返し | `$(...),*`、可変個の引数を受け取るマクロ |
| ch23-03 | unsafe と生ポインタ | `unsafe` ブロックでできること、未定義動作とは何か、`*const T`・`*mut T`、`&raw const`・`&raw mut` |
| ch23-04 | unsafe を安全な API で包む | `unsafe fn`（edition 2024 では中でも `unsafe` ブロックが必要）、安全性の前提を守る抽象化、`static mut` を避ける理由と代わりの手段（ch20-09・ch20-10） |
| ch23-review | 章末復習 | |
