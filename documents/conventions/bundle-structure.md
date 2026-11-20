---
type: Convention
title: "ドキュメントの置き場所と構成"
description: "ドキュメントを `documents/` に集約すること、`spec/`・`decisions/`・`conventions/` の役割、asset（`.md` 以外のファイル）の扱い、`log.md`・`README.md` を置かないこと、一時作業用ディレクトリを参照しないことを定める。"
---

- 仕様、設計判断記録（ADR）、ドキュメントの運用ルールは、すべて `documents/` に置く。`documents/` は OKF（Open Knowledge Format）v0.3 のバンドルで、`documents/` がバンドルのルートになる。
- バンドルのルートには `index.md` と、次の3つのディレクトリを置く。

| ディレクトリ | 入る文書 |
| --- | --- |
| `spec/` | 仕様。現時点の事実を書く。書き方は [仕様の書き方](spec-writing.md) |
| `decisions/` | 設計判断記録（ADR）。仕様の「なぜ」を書く。書き方は [ADR の運用ルール](decision-records.md) |
| `conventions/` | ドキュメント自体の書き方と運用ルール |

```
documents/
  index.md                ← frontmatter を持つ index.md はここだけ
  spec/
    index.md
    overview.md
    glossary/
    qa-chat/              ← 機能ごとに1ディレクトリ
      index.md
      scope.md
      ...
  decisions/
    index.md
    0001-....md
  conventions/
    index.md
    ...
```

## 置いてよいファイル

- `documents/` の中の `.md` は、concept（frontmatter に `type` を持つ文書）か `index.md` のどちらかにする。concept の frontmatter は [frontmatter の書き方](frontmatter.md)、`index.md` は [index.md の書き方と更新](index-files.md) に従う。
- `log.md` と `README.md` は置かない。
- `.md` 以外のファイルは asset になる。asset は concept ではなく、`index.md` に載せない。asset は本文からリンクで参照する。
  - 例: 学習記録のテンプレートは `spec/learning-record-and-analysis/templates/record.md.tmpl` に置き、[学習記録ファイル](../spec/learning-record-and-analysis/record-file.md) の本文からリンクしている。
- 名前が `.` で始まるファイルとディレクトリは、バンドルに含まれない。

## 一時作業用ディレクトリ

- リポジトリの一時作業用ディレクトリ（`CLAUDE.md` の「その他」にあるもの）の中のファイルを、`documents/` の文書から参照しない。必要な内容は、`documents/` の中の文書に書く。
