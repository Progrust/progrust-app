---
type: Specification
title: "AI モデルの選択"
description: "設定画面で用途ごとに選ぶモデルの設定、既定値、選択肢、設定の保存先を定める。"
---

設定画面で、用途ごとにモデルを選ぶ。

| 設定 | 用途 | 既定値 |
|---|---|---|
| 質疑応答のモデル | 質疑応答、接続テスト | `sonnet` |
| 記録・分析・問題生成のモデル | すべてのジョブ | `opus` |

- 選択肢は `opus` / `sonnet` / `haiku`（Claude Code のモデルのエイリアス）と「その他（モデル ID を入力）」とする。
- 設定は `settings.json` に保存する（[設定画面と settings.json](../screens/settings-screen.md)）。
