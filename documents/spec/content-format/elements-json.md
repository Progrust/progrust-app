---
type: Specification
title: "要素マスタの形式（elements.json）"
description: "`elements.json` の各要素のフィールド（`id`・`chapter`・`name`・`description`・`related_errors`）と、要素の分け方と各フィールドの決め方の参照先、要素 ID を廃止するときに `deprecated` を付けることを定める。"
---

```json
[
  {
    "id": "ownership.move",
    "chapter": "ch04",
    "name": "所有権のムーブ",
    "description": "変数が値そのものを持つこと、代入で所有権が移ること、ムーブ済みの値を使えないこと",
    "related_errors": ["E0382"]
  }
]
```

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | `{分類}.{要素}` 形式の snake_case。要素マスタの中で重複させない。一度公開したら変更しない |
| `chapter` | string | [章とレッスンの一覧](../curriculum/chapters/index.md) の学習順で、最初にその要素を「主要素」とするレッスンが属する章の ID |
| `name` | string | 表示名 |
| `description` | string | 要素の説明（AI への入力・生成設定画面の説明に使用） |
| `related_errors` | string[] | その要素の誤解で発生しやすいコンパイルエラーコード |

- 要素の分け方、要素 ID の付け方と分類の一覧、`name`・`description`・`related_errors` の決め方は [要素の分け方と要素 ID の付け方](../content-guidelines/elements.md) に定める。
- `related_errors` は、試行ログ中のエラーコードから躓いた要素の候補を出すために使う（[学習記録・分析の AI ジョブ](../learning-record-and-analysis/ai-jobs.md)「記録セッション」）。
- 要素 ID を廃止する場合は、削除せず `"deprecated": true` を付ける。
