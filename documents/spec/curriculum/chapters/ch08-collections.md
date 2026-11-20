---
type: Specification
title: "ch08 コレクション"
description: "ch08 コレクションの章の説明と、Vec の作成・走査・並べ替えと検索、pop と while let、文字列の連結・分割と String と UTF-8、HashMap の基本と更新、HashSet と BTreeMap を扱うレッスン（ch08-01〜ch08-10 と章末復習）の ID・タイトル・主に扱う内容を定める。"
---

ベクタ・文字列・ハッシュマップなど、標準ライブラリのコレクションを学ぶ

| ID | タイトル | 主に扱う内容 |
| --- | --- | --- |
| ch08-01 | Vec の作成と要素の読み取り | `Vec::new`、`vec!`、`push`、`len`、添字と `get` の違い、要素の参照を持ったままの `push` が借用のエラーになること |
| ch08-02 | Vec の走査とスライス | `for` で `&v`・`&mut v` を回す、要素の書き換え、`Vec` の一部を `&[T]` で貸す |
| ch08-03 | pop と while let | 末尾から取り出す、`while let` によるループ |
| ch08-04 | Vec の並べ替えと検索 | `sort`、`dedup`、`contains`、`reverse` |
| ch08-05 | 文字列の連結と結合 | `+` 演算子と所有権、`join`・`concat` で `Vec<String>` をつなぐ |
| ch08-06 | 文字列を調べて分割する | `split`・`split_whitespace`・`lines`・`trim`・`contains`・`starts_with`・`replace`・`to_uppercase` |
| ch08-07 | String と UTF-8 | 添字でアクセスできない理由、`chars`・`bytes`、バイト数と文字数の違い、`char` の `is_ascii_digit`・`to_digit`・`is_alphabetic` |
| ch08-08 | HashMap の基本 | `use` で標準ライブラリの型を取り込む、`insert`・`get`、走査の順序が決まっていないこと、キーと値の所有権 |
| ch08-09 | HashMap の更新 | `entry`・`or_insert`、出現回数を数える |
| ch08-10 | HashSet と BTreeMap | 重複のない集合、キーの順に並ぶマップ（出力の順序を決めたいとき） |
| ch08-review | 章末復習 | |
