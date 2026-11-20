---
type: Decision
title: "ADR 0027"
description: "学習記録と分析ファイルを Markdown + YAML frontmatter で保存する"
---

- 日付: 2026-10-31
- 関連仕様: [学習記録・分析の設計方針](../spec/learning-record-and-analysis/design-principles.md)「ファイル形式」、[学習記録ファイル（records/*.md）](../spec/learning-record-and-analysis/record-file.md)、[分析ファイル（analysis/analysis.md）](../spec/learning-record-and-analysis/analysis-file.md)、[学習記録・分析ファイルの手動編集への対応](../spec/learning-record-and-analysis/manual-edit.md)、[トップページ（ダッシュボード）](../spec/screens/top-page.md)
- 置き換え: なし

## 背景

学習記録・分析ファイルは、アプリが機械的に読み取る（ダッシュボード・問題生成・集計）だけでなく、AI が読み書きし、学習者も読む。もともとの要件で、学習記録は Markdown 形式とし、テンプレートも用意することにしていた。

## 決定

構造化データを YAML frontmatter に、人と AI が読む文章を Markdown の本文に書く。アプリは本文を解析しない（`learning-record-and-analysis.md`「ファイル形式」）。

## 検討した他の案

- 案: JSON で保存する
  - 採用しなかった理由: アプリには扱いやすいが、学習者が読みにくく、AI が書く文章の表現に向かない
- 案: Markdown の本文だけにし、アプリが本文を解析する
  - 採用しなかった理由: AI の書き方の揺れで解析が壊れやすい

## 影響

- 学習者がエディタで開いて読んだり、手で編集したりできる。手で壊された場合への対応が必要になる。
- 本文は解析しないため、画面に出す情報は frontmatter に持たせる必要がある（例外は総評の表示のみ）。
