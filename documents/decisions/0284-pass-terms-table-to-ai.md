---
type: Decision
title: "ADR 0284"
description: "訳語の表を質疑応答と演習問題生成の AI に渡し、回答と生成問題で教材と同じ語を使わせる"
---

- 日付: 2026-11-28
- 関連仕様: [質疑応答で AI に渡す入力](../spec/qa-chat/ai-input.md)、[質疑応答の回答の方針](../spec/qa-chat/answer-policy.md)「共通」、[演習問題生成で AI に渡す入力](../spec/exercise-generation/ai-input.md)
- 置き換え: なし

## 背景

教材の語を訳語の表でそろえることにした（[ADR 0283](0283-share-readings-and-terms-tables-across-lessons.md)）。質疑応答の AI に表を渡さないと、動画では「共有参照」と言い、チャットでは「不変参照」と答える、という食い違いが起きる。Claude がこのことを伝え、ユーザーは、質疑応答の AI にも渡すことにした。

## 決定

- 質疑応答の AI に、動画の再生中と解説表示中の両方で、訳語の表を渡す（[質疑応答で AI に渡す入力](../spec/qa-chat/ai-input.md)）。C のレッスンにも渡す。
- 回答では表の語を使い、学習者が使わない語で質問したときは、表の語に言い換え、同じものを指すことを一度だけ添える（[質疑応答の回答の方針](../spec/qa-chat/answer-policy.md)「共通」）。以下の理由は Claude が書いた。
  - 理由: 学習者は、ほかの資料で「不変参照」のような語に出会う。言い換えるだけだと、別のものの話だと思われるおそれがある。
- 演習問題生成の AI にも、訳語の表を渡す（[演習問題生成で AI に渡す入力](../spec/exercise-generation/ai-input.md)）。これは Claude が提案し、ユーザーが承認した。
  - 理由: 生成問題の問題文と解説も、学習者が読む教材の文である。

## 検討した他の案

- 案: 質疑応答の AI には渡さない
  - 採用しなかった理由: 教材とチャットで語が食い違う

## 影響

- AI への入力が、訳語の表の分だけ長くなる。
- 指示文（`prompts/chat-video.md`・`prompts/chat-explanation.md`・`prompts/generate.md`）と入力の組み立ての実装が要る。
