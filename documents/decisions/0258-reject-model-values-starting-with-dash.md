---
type: Decision
title: "ADR 0258"
description: "`settings.json` の AI のモデルの値は、空でなく、空白と制御文字を含まず、`-` で始まらないものだけを受け付ける"
---

- 日付: 2026-11-27
- 関連仕様: [設定画面と settings.json](../spec/screens/settings-screen.md)「settings.json」、[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)
- 置き換え: なし

## 背景

設定画面では、AI のモデルを `opus`・`sonnet`・`haiku` のほか、「その他（モデル ID を入力）」で選べる（[AI モデルの選択](../spec/claude-integration/model-selection.md)）。選んだ値は、`claude -p` の `--model` の値として渡す（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)）。仕様には、モデルの値の条件が無かった。いまの実装は、空の値と、空白・制御文字を含む値を受け付けない。

`-` で始まる値を `--model` の後ろに渡すと、`claude` が値ではなく別のフラグとして読むおそれがある。そうなると、隔離のために付けたフラグ（[ADR 0106](0106-isolated-claude-flags-and-oauth-only.md)）と違う条件で `claude` が動くことがある。

M4 の実装計画で、Claude が技術的な判断で決めた。

## 決定

- `models.chat`・`models.background` は、空でなく、空白と制御文字を含まず、`-` で始まらない文字列だけを受け付ける（[設定画面と settings.json](../spec/screens/settings-screen.md)「settings.json」）。満たさない値は、`update_settings` が `invalid_argument` を返す。
  - 理由: モデルのエイリアスとモデル ID は、どれも英字で始まる。`-` で始まる値を受け付けなければ、`--model` の値が別のフラグとして読まれることがない。
- `claude -p` を起動する前にも、モデルの値がこの条件を満たすかを確かめ、満たさないときは起動せずに、その他のエラーとする（[claude -p の呼び出しの共通ルール](../spec/claude-integration/invocation.md)）。
  - 理由: `settings.json` を読むときは、値の条件を確かめていない。学習者が `settings.json` を手で書き換えても、条件を満たさない値が `--model` に渡らないようにする。

## 検討した他の案

- 案: 値を `--model=<値>` の形で 1 つの引数として渡し、値の条件は付けない
  - 採用しなかった理由: この形での読み方を、Claude Code の版ごとに確かめる必要がある。仕様の起動のフラグの書き方とも変わる
- 案: モデル ID に使える文字（英数字と一部の記号）だけを受け付ける
  - 採用しなかった理由: モデル ID の形は Claude Code とモデルの提供のしかたで変わり、許す文字を決めきれない。新しいモデル ID が使えなくなるおそれがある

## 影響

- 手で書き換えた `settings.json` のモデルの値が条件を満たさないときは、AI の呼び出しが「AI の呼び出しに失敗しました」になる。設定画面でモデルを選び直せば直る。
