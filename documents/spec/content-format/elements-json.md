---
type: Specification
title: "要素マスタの形式（elements.json）"
description: "`elements.json` の各要素のフィールド（`id`・`chapter`・`name`・`description`・`related_errors`）と、要素 ID を廃止するときに `deprecated` を付けることを定める。"
---

```json
[
  {
    "id": "ownership.move",
    "chapter": "ch04",
    "name": "所有権のムーブ",
    "description": "値の代入・関数呼び出しによる所有権の移動",
    "related_errors": ["E0382", "E0505"]
  }
]
```

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | `{分類}.{要素}` 形式の snake_case。一度公開したら変更しない |
| `chapter` | string | その要素を主に扱う章の ID |
| `name` | string | 表示名 |
| `description` | string | 要素の説明（AI への入力・生成設定画面の説明に使用） |
| `related_errors` | string[] | その要素の誤解で発生しやすいコンパイルエラーコード |

- `related_errors` は、試行ログ中のエラーコードから躓いた要素の候補を出すために使う（[学習記録・分析の AI ジョブ](../learning-record-and-analysis/ai-jobs.md)「記録セッション」）。
- 要素 ID を廃止する場合は、削除せず `"deprecated": true` を付ける。
