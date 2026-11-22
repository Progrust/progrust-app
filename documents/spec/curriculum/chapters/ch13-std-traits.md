---
type: Specification
title: "ch13 標準ライブラリのトレイト"
description: "ch13 標準ライブラリのトレイトの章の説明と、複製と既定値（`Clone`・`Copy`・`Default`）、比較とハッシュ、Display の実装、FromStr の実装、From と Into、演算子のオーバーロードを扱うレッスン（ch13-01〜ch13-06 と章末復習）の ID・タイトル・主に扱う内容・主要素を定める。"
---

標準ライブラリのトレイトを実装・derive して、型を標準の機能と組み合わせる方法を学ぶ

| ID | タイトル | 主に扱う内容 | 主要素 |
| --- | --- | --- | --- |
| ch13-01 | 複製と既定値 | `Clone`・`Copy`・`Default` の derive と、構造体を `Copy` にできる条件 | `std_trait.clone_copy_default` |
| ch13-02 | 比較とハッシュ | `PartialEq`・`Eq`・`Hash`・`PartialOrd`・`Ord`、`Ordering`、`HashMap` のキーと `sort` に使える型 | `std_trait.eq_ord_hash` |
| ch13-03 | Display を実装する | `std::fmt::Display`、`{}` で出力できるようにする、`to_string` | `std_trait.display` |
| ch13-04 | FromStr を実装する | `parse` で独自の型に変換できるようにする、`type Err` | `std_trait.from_str` |
| ch13-05 | From と Into | 型の変換をトレイトで表す、`From` を実装すると `Into` が使える、`TryFrom`・`TryInto` | `std_trait.from_into` |
| ch13-06 | 演算子のオーバーロード | `std::ops::Add` など、`type Output`、既定の型パラメータ（`Rhs = Self`） | `std_trait.ops` |
| ch13-review | 章末復習 | | |
