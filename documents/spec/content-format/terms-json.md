---
type: Specification
title: "訳語の表の形式（terms.json）"
description: "教材で使う訳語の表（`content/terms.json`）の各項目のフィールド（`term`・`code`・`avoid`・`note`）と、訳語の決め方の参照先を定める。"
---

訳語の表は、教材で使う語と、使わない語（言葉のゆれ）の対応を、全レッスン共通で持つ。アプリに同梱し、質疑応答と演習問題生成の AI にも渡す（[質疑応答で AI に渡す入力](../qa-chat/ai-input.md)、[演習問題生成で AI に渡す入力](../exercise-generation/ai-input.md)）。

```json
[
  {
    "term": "共有参照",
    "code": "&T",
    "avoid": ["不変参照", "不変の参照", "イミュータブル参照"],
    "note": "コンパイラの出力の immutable borrow は、共有参照として借用していることだと説明する"
  }
]
```

| フィールド | 型 | 説明 |
|---|---|---|
| `term` | string | 教材で使う語。表の中で重複させない |
| `code` | string（省略可） | その語が指すコードの書き方 |
| `avoid` | string[] | 使わない語。無ければ空の配列 |
| `note` | string（省略可） | 使い方の補足 |

- 訳語の決め方と、表に語を足すときの決まりは [教材で使う訳語](../content-guidelines/terms.md) に定める。
