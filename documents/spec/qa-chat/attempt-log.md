---
type: Specification
title: "質疑応答の試行ログへの記録"
description: "質問を送信したときに試行ログへ `chat` イベントを追記する条件を定める。"
---

- 質問を送信したとき、試行ログに `chat` イベントを追記する（スキーマは [試行ログ（attempts/*.json）](../learning-record-and-analysis/attempt-log.md)「スキーマ」）。
- 試行セッションが存在する場面（レッスンの動画画面、レッスン・生成問題の演習画面）でのみ記録する。
