---
type: Specification
title: "学習記録・分析の目的とスコープ"
description: "学習記録・分析の仕様が扱う範囲（対象は Rust の演習で、C の演習は対象にしないこと）と、`claude -p` の呼び出し方とジョブキュー、データの使い道の参照先を定める。"
---

Claude Code 連携機能のうち「学習記録」「分析」に関する仕様を定める。
学習記録と分析の対象は、Rust の演習（Rust のレッスンの演習と生成問題）とする。C の演習は対象にしない。試行ログは、C のレッスンでも記録する（[試行ログ（attempts/*.json）](attempt-log.md)）。
`claude -p` の呼び出し方とジョブキューは [Claude Code 連携](../claude-integration/index.md) に、本仕様のデータを画面や問題生成でどう使うかは [画面仕様](../screens/index.md)・[演習問題生成](../exercise-generation/index.md) に定める。
