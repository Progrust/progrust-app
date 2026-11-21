---
type: Specification
title: "章とレッスンの一覧の読み方"
description: "章ごとのファイルの `title`・本文の最初の文・表の並びと、`index.md` の章の並びが表すもの（章 ID と章のタイトル、`chapters.json` の `description`、学習順）と、章末復習レッスンのタイトルを一覧では定めないこと、動画のみのレッスンの印の付け方を定める。"
---

章ごとのファイルの `title` は「章 ID + 章のタイトル」、本文の最初の文は章の説明（`chapters.json` の `description`）である。`index.md` に並べた章の順が章の学習順、表の並びが章の中の学習順である。
章末復習レッスンのタイトルはここでは定めず、表では「章末復習」と書く。
動画のみのレッスン（[レッスン定義の形式（lesson.json）](../../content-format/lesson-json.md) の `kind` が `video_only` のレッスン）は、「主に扱う内容」の末尾に「（動画のみのレッスン）」と書く。この印のないレッスンは、動画のみのレッスンにしない。
