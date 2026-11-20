---
type: Specification
title: "Claude Code 連携の設計方針"
description: "AI にツールを使わせないこと、数値の計算をさせないこと、AI 機能が使えなくても学習を続けられることを定める。"
---

- AI にはファイルの編集・コマンドの実行などのツールを使わせない。結果は標準出力で返させ、アプリが検証してから保存する。
- AI に数値・日時・回数を計算させない（[学習記録・分析の設計方針](../learning-record-and-analysis/design-principles.md)「事実と解釈の分離」）。
- AI 機能が使えなくても、学習（動画・演習・完了）は続けられるようにする。
