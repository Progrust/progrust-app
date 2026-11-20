---
type: Specification
title: "Claude Code 連携の目的とスコープ"
description: "Claude Code 連携の共通仕様が扱う範囲と、機能ごとの呼び出し方（直接呼び出しかジョブか）および仕様書の対応を定める。"
---

`claude -p` の呼び出し方、モデルの選択、ジョブキューなど、AI を使うすべての機能に共通するルールを定める。
各機能の入力・出力・検証は、それぞれの仕様書に定める。

| 機能 | 呼び出し方 | 仕様書 |
|---|---|---|
| 質疑応答 | 画面から直接呼び出す | [質疑応答](../qa-chat/index.md) |
| 学習記録・分析・分析の再構築 | ジョブ | [学習記録・分析](../learning-record-and-analysis/index.md) |
| 演習問題生成 | ジョブ | [演習問題生成](../exercise-generation/index.md) |
