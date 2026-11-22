---
type: Specification
title: "ch07 列挙型とパターンマッチ"
description: "ch07 列挙型とパターンマッチの章の説明と、列挙型と match、データを持つバリアント、Option 型、パターンの種類、構造体とタプルの分解、参照とパターン、マッチガードと @ 束縛、if let と let else を扱うレッスン（ch07-01〜ch07-08 と章末復習）の ID・タイトル・主に扱う内容・主要素を定める。"
---

データを持つ列挙型と、パターンによる値の分解・分岐を学ぶ

| ID | タイトル | 主に扱う内容 | 主要素 |
| --- | --- | --- | --- |
| ch07-01 | 列挙型と match | 列挙型の定義、`match` による分岐、網羅性の検査、`_` | `enum.basic`、`pattern.match` |
| ch07-02 | データを持つバリアント | バリアントにデータを持たせる、`match` でデータを取り出す、列挙型のメソッド | `enum.data_variant` |
| ch07-03 | Option 型 | null がないこと、`Some`・`None`、`Option<T>` と `T` の区別、`is_some`・`is_none`、`checked_add` など `Option` を返す演算 | `option.basic` |
| ch07-04 | パターンの種類 | リテラル、`\|`、範囲 `..=`、変数への束縛、`..` による残りの無視 | `pattern.kinds` |
| ch07-05 | 構造体とタプルの分解 | 入れ子の分解、`let` と関数の引数でのパターン | `pattern.destructure` |
| ch07-06 | 参照とパターン | 参照に対する `match`（match ergonomics）、`&x` パターン、`ref`・`ref mut` | `pattern.reference` |
| ch07-07 | マッチガードと @ 束縛 | `if` 条件付きのアーム、`name @ パターン` | `pattern.guard_at` |
| ch07-08 | if let と let else | 1 つのパターンだけを扱う書き方、論駁可能なパターンと論駁不可能なパターン、let チェーン（`if let ... && ...`） | `pattern.if_let` |
| ch07-review | 章末復習 | | |
