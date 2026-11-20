---
type: Decision
title: "ADR 0084"
description: "学習記録・分析のテンプレートを、`.md.tmpl` の asset としてバンドルに置く"
---

- 日付: 2026-11-20
- 関連仕様: [データの保存場所](../spec/architecture/data-locations.md)、[学習記録ファイル（records/*.md）](../spec/learning-record-and-analysis/record-file.md)、[分析ファイル（analysis/analysis.md）](../spec/learning-record-and-analysis/analysis-file.md)
- 置き換え: なし

## 背景

学習記録と分析のテンプレート 2 つは、移行前は仕様書のディレクトリの `templates/` に `record.md`、`analysis.md` として置いていた。この 2 つは `type` を持たない `.md` で、frontmatter に `{{...}}` の埋め込み記号がある。ドキュメントを OKF v0.3 のバンドルにするにあたり（0079）、バンドルの中に `.md` のまま置くと、concept でも `index.md` でもないファイルになる。`documents/` 配下の `.md` は concept か `index.md` だけにすると決めているため、置き方を決める必要があった。

## 決定

- 2 つのテンプレートを `documents/spec/learning-record-and-analysis/templates/` に `record.md.tmpl`、`analysis.md.tmpl` として置く。OKF では `.md` 以外のファイルは asset になる。
- アプリに同梱するときのファイル名（`templates/record.md`、`templates/analysis.md`）は変えない。
- 以上は Claude の提案で、ユーザーが「すべて提案通りで大丈夫です。」「「決めてほしい点」もすべてあなたの推奨通りで問題ないので、この方針でドキュメントの整理をお願いします。」と承認した。

## 検討した他の案

- なし

## 影響

- リポジトリ上の原本のファイル名（`.md.tmpl`）と、アプリに同梱するときのファイル名（`.md`）が異なる。同梱するときに名前を変える。
