---
type: Specification
title: "カリキュラムの目的とスコープ"
description: "カリキュラムの仕様が扱う範囲と、章とレッスンの ID・タイトル・章の説明・学習順・各レッスンの主要素をこの仕様を唯一の定義とすること、副要素と章末復習レッスンの主要素は `lesson.json` で定めること、教材の作成ルールと `chapters.json` の形式の参照先を定める。"
---

章立てと学習順、各レッスンで主に扱う内容と主要素を定める。
教材の作成ルールは [教材作成ガイドライン](../content-guidelines/index.md)、アプリが読む章とレッスンの並びの形式は [章とレッスンの並びの形式（chapters.json）](../content-format/chapters-json.md) に定める。

- 作成する章とレッスン、その学習順、各レッスンで主に扱う内容と主要素を定める。
- 章とレッスンの ID・タイトル、章の説明、学習順、各レッスン（動画のみのレッスンと章末復習レッスンを除く）の主要素は、この仕様書を唯一の定義とする。`content/chapters.json` と、各レッスン（章末復習レッスンを除く）の `lesson.json` の `title`・`primary_elements` は、これに合わせる（[カリキュラムの受け入れ条件](acceptance.md)）。
- 主要素に書く要素（要素 ID）は、要素マスタ（[要素マスタの形式（elements.json）](../content-format/elements-json.md)）に定めたものとする。要素の分け方は [要素の分け方と要素 ID の付け方](../content-guidelines/elements.md) に従う。
- 各レッスンの副要素と、章末復習レッスンの主要素は、レッスンを作るときに `lesson.json` に定める。動画のみのレッスンは、主要素も副要素も持たない。
