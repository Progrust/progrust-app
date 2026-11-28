---
type: Specification
title: "章とレッスンの一覧の読み方"
description: "章ごとのファイルの `title`・本文の最初の文・表の並びと、`index.md` の章の並びが表すもの（章 ID と章のタイトル、`chapters.json` の `description`、学習順）と、章 ID の形から章の言語が決まること、表の「主要素」の列の書き方と C の章の表に「主要素」の列が無いこと、章末復習レッスンのタイトルと主要素を一覧では定めないこと、動画のみのレッスンの印の付け方を定める。"
---

章ごとのファイルの `title` は「章 ID + 章のタイトル」、本文の最初の文は章の説明（`chapters.json` の `description`）である。`index.md` に並べた章の順が章の学習順、表の並びが章の中の学習順である。
章 ID が `c{NN}` の章は C の章、`ch{NN}` の章は Rust の章である（[章とレッスンの並びの形式（chapters.json）](../../content-format/chapters-json.md) の `language`）。`index.md` には、C の章を、すべての Rust の章より前に並べる。
表の「主要素」の列は、そのレッスンの主要素（`lesson.json` の `primary_elements`）を、要素 ID（[要素マスタの形式（elements.json）](../../content-format/elements-json.md)）で表す。2 つあるときは「、」で区切って並べ、`primary_elements` にも同じ順に書く。動画のみのレッスンは主要素を持たないので、空欄にする。
C の章の表には「主要素」の列が無い。C のレッスンは主要素を持たない（[レッスン定義の形式（lesson.json）](../../content-format/lesson-json.md)「C のレッスン」）。
章末復習レッスンのタイトルと主要素はここでは定めず、表ではタイトルを「章末復習」と書き、「主要素」を空欄にする。Rust の章の章末復習レッスンの主要素の選び方は [演習の作成ルール](../../content-guidelines/exercise.md)「章末復習レッスン」に定める。
動画のみのレッスン（[レッスン定義の形式（lesson.json）](../../content-format/lesson-json.md) の `kind` が `video_only` のレッスン）は、「主に扱う内容」の末尾に「（動画のみのレッスン）」と書く。この印のないレッスンは、動画のみのレッスンにしない。
