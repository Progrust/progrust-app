---
type: Decision
title: "ADR 0032"
description: "躓きの pattern ラベルを、既存のラベル一覧から再利用させる"
---

- 日付: 2026-10-31
- 関連仕様: [学習記録ファイル（records/*.md）](../spec/learning-record-and-analysis/record-file.md)「frontmatter」
- 置き換え: なし

## 背景

同じ種類の躓きが繰り返されているか（再発）を判定するには、記録ごとに躓きの種類を同じラベルで表す必要がある。記録ごとに AI が自由にラベルを付けると、同じ躓きに別々のラベルが付き、再発を判定できない。

## 決定

記録セッションに、その要素について過去に使われた pattern ラベルの一覧を渡し、同じ種類の躓きには既存のラベルを再利用させる（`learning-record-and-analysis.md`「frontmatter」の「pattern ラベルの一貫性」）。

## 検討した他の案

- 案: ラベルの一覧をあらかじめ定義しておく
  - 採用しなかった理由: 躓きの種類を事前に網羅できず、新しい躓きを表せない

## 影響

- 再発（`recurring_patterns`）を機械的に判定できる。
- AI が既存ラベルを再利用しない場合は、再発を見逃す。
