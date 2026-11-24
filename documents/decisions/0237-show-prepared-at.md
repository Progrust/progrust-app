---
type: Decision
title: "ADR 0237"
description: "環境の状態の初回準備に、準備が終わった日時 prepared_at を足し、設定画面に「準備できています」と日時を表示する"
---

- 日付: 2026-11-24
- 関連仕様: [環境のコマンドとイベント](../spec/architecture/commands/environment.md)「環境の状態」、[設定画面と settings.json](../spec/screens/settings-screen.md)「設定項目」
- 置き換え: なし

## 背景

画面デザインの設定画面（`design-system/ui-design/Settings.dc.html`）は、外部クレートの準備の項目に「準備できています（10/28 19:02）」と、準備が終わった日時を表示している。一方で、環境の状態の `crates` には `status` と `error` しかなく、画面が日時を知る方法が無かった。

## 決定

以下は、Claude の技術的な判断である。

- 環境の状態の `crates` に `prepared_at` を足す（[環境のコマンドとイベント](../spec/architecture/commands/environment.md)「環境の状態」）。値は `prepared.json` の `prepared_at` で、`ready` のときだけ持つ。
  - 理由: 画面デザインのとおりに表示できる。いつ準備したかが分かると、`rustc` を更新した後に準備がやり直されたことを、学習者が確かめられる。

## 検討した他の案

- 案: 画面デザインから日時を消す
  - 採用しなかった理由: 足すのは値 1 つで、作る手間が小さい

## 影響

- なし
