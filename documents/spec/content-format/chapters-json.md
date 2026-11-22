---
type: Specification
title: "章とレッスンの並びの形式（chapters.json）"
description: "`chapters.json` の形式と、学習順・過去のレッスン・導入レッスン・既習の要素・準備中のレッスンの定義、各章の最後に章末復習レッスン（`ch{NN}-review`）を置くことを定める。"
---

[章とレッスンの一覧](../curriculum/chapters/index.md) に定めた章立てと学習順を、アプリが読む形式で表したものとする。内容は [章とレッスンの一覧](../curriculum/chapters/index.md) と一致させる（教材チェックツールで検証する: [教材チェックツール](../content-guidelines/check-tool.md)）。

```json
[
  {
    "id": "ch04",
    "title": "所有権",
    "description": "Rust のメモリ管理の中心となる所有権の仕組みを学ぶ",
    "lessons": ["ch04-01", "ch04-02", "ch04-03", "ch04-review"]
  }
]
```

- 配列の順序が章の学習順、`lessons` の順序が章内の学習順である。
- 全レッスンをこの順に一列に並べたものを「学習順」と呼ぶ。「過去のレッスン」は学習順で前にあるレッスンを指す。
- 要素の「導入レッスン」は、学習順で最初にその要素を `primary_elements` に含むレッスンとする。
- あるレッスンにとっての「既習の要素」は、導入レッスンが学習順でそのレッスンより前にある要素とする。
- 各章の最後のレッスンは章末復習レッスン（`ch{NN}-review`）とする。

## 準備中のレッスン

- `chapters.json` にあるレッスンのうち、レッスンのディレクトリ（`content/lessons/{lesson_id}/`。[教材のディレクトリ構成](directory-structure.md)）が無いものを「準備中のレッスン」とする。
- ディレクトリがあり、必要なファイルが欠けているレッスンは、準備中のレッスンではない。教材チェックツールの「構成」のエラーになる（[教材チェックツール](../content-guidelines/check-tool.md)）。
- 学習順には、準備中のレッスンも含む。
