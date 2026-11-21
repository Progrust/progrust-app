---
type: Acceptance Criteria
title: "カリキュラムの受け入れ条件"
description: "`content/chapters.json` と各レッスンの `lesson.json` の `title` が章とレッスンの一覧と一致すること、動画のみのレッスンが一覧の定めと一致すること、扱わない内容を主要素とするレッスンがないことを、カリキュラムの仕様を満たしたと判断する条件として定める。"
---

- `content/chapters.json` の章の並び、各章の ID・タイトル・説明、各章のレッスンの並びが [章とレッスンの一覧](chapters/index.md) と一致する（教材チェックツールで検証する: [教材チェックツール](../content-guidelines/check-tool.md)）。
- 章末復習レッスン以外の各レッスンの `lesson.json` の `title` が [章とレッスンの一覧](chapters/index.md) のタイトルと一致する（同上）。
- `lesson.json` の `kind` が `video_only` のレッスンが、[章とレッスンの一覧](chapters/index.md) で動画のみのレッスンと定めたレッスンと一致する（同上）。
- [カリキュラムで扱わない内容](out-of-scope.md) の内容を主要素とするレッスンがない。
