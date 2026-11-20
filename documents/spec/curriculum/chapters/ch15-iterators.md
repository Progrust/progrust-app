---
type: Specification
title: "ch15 イテレータ"
description: "ch15 イテレータの章の説明と、Iterator トレイトと next、iter・iter_mut・into_iter、map と filter、collect と消費アダプタ、cloned と copied、並びを組み替えるアダプタ、要素を探す方法、filter_map と flat_map、クロージャで並べ替える方法、イテレータを返す関数、独自のイテレータの実装を扱うレッスン（ch15-01〜ch15-11 と章末復習）の ID・タイトル・主に扱う内容を定める。"
---

イテレータとアダプタによる繰り返し処理を学ぶ

| ID | タイトル | 主に扱う内容 |
| --- | --- | --- |
| ch15-01 | Iterator トレイトと next | `next` と `Option`、`type Item`、`for` がイテレータを使っていること、遅延評価 |
| ch15-02 | iter・iter_mut・into_iter | 要素を参照・可変参照・所有権のどれで取り出すか |
| ch15-03 | map と filter | イテレータアダプタ、アダプタだけでは何も起きないこと、`filter` のクロージャが参照の参照を受け取ること（`\|&x\|`・`**x`） |
| ch15-04 | collect と消費アダプタ | `collect`（`Vec`・`String`・`HashMap` へ）、`sum`・`count`・`min`・`max`・`last`・`fold` |
| ch15-05 | cloned と copied | `Vec<&T>` と `Vec<T>` の違い、参照の要素を値にする |
| ch15-06 | 並びを組み替えるアダプタ | `enumerate`・`zip`・`chain`・`rev`・`take`・`skip`・`take_while`・`skip_while`・`step_by` |
| ch15-07 | 要素を探す | `any`・`all`・`find`・`position`・`max_by_key`・`min_by_key` |
| ch15-08 | filter_map と flat_map | 変換と絞り込みを同時に行う、入れ子を平らにする |
| ch15-09 | クロージャで並べ替える | `sort_by`・`sort_by_key`、`cmp` と `Ordering`、`then_with` |
| ch15-10 | イテレータを返す関数 | `impl Iterator<Item = T>` を返す、引数の参照を使うイテレータを返す（edition 2024 では `+ '_` が不要） |
| ch15-11 | 独自のイテレータを実装する | 構造体に `Iterator` を実装し、アダプタを使えるようにする |
| ch15-review | 章末復習 | |
