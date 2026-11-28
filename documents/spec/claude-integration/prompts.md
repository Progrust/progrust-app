---
type: Specification
title: "指示文（プロンプト）の置き場所と書き方"
description: "指示文を `prompts/` に Markdown で置くこと、`{{name}}` 形式の埋め込み、指示文ファイルと用途の対応、レッスンの言語（C か Rust）でファイルを分けずに同じファイルに言語に応じた指示を書くことを定める。"
---

- 各機能の指示文はリポジトリの `prompts/` に Markdown で置き、アプリに同梱する。
- 指示文に埋め込む値は `{{name}}` 形式で書き、アプリが置き換える。
- 指示文に何を含めるか（入力の項目）は各機能の仕様書に定める。指示文の文面は実装の一部として扱い、仕様書には書かない。

| ファイル | 用途 |
|---|---|
| `prompts/record.md` | 記録セッション |
| `prompts/analysis.md` | 分析セッション |
| `prompts/generate.md` | 問題生成 |
| `prompts/generate-fix.md` | 検証に失敗した生成問題の修正の依頼 |
| `prompts/chat-video.md` | 動画についての質疑応答 |
| `prompts/chat-explanation.md` | 解説表示後の質疑応答 |

- 指示文のファイルは、用途ごとに 1 つとし、レッスンの言語（C か Rust）で分けない。質疑応答の C のレッスンと Rust のレッスンで変わる指示（[質疑応答の回答の方針](../qa-chat/answer-policy.md)）は、同じファイルの中に、レッスンの言語に応じた指示として書く。
