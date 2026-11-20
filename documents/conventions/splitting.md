---
type: Convention
title: "文書の分割単位と名前の付け方"
description: "1つの concept に1つのトピックを書くこと、約200行を超えるときの分け方、機能のディレクトリに置く `scope.md`・`acceptance.md`・`open-questions.md`、用語集と `overview.md` の単位、ファイル名・`title`・`description` の付け方、本文の見出しの書き方を定める。"
---

## 分割の単位

- 1つの concept には、1つのトピックだけを書く。
- 1つのトピックが約200行を超えるときは、その中の1段下の見出しごとに concept を分け、トピックをディレクトリにする。そのディレクトリにも `index.md` を置く。
- 仕様は、機能ごとに1ディレクトリにする（例: `spec/qa-chat/`）。機能のディレクトリには、トピックごとの concept と、次のファイルを置く。

| ファイル | type | 置くとき | 内容 |
| --- | --- | --- | --- |
| `scope.md` | `Specification` | 常に | 機能の目的とスコープ。title は「〈機能名〉の目的とスコープ」 |
| `acceptance.md` | `Acceptance Criteria` | 受け入れ条件を書くとき | 機能の受け入れ条件。機能のディレクトリごとに1つにまとめ、トピックごとのファイルに分けない |
| `open-questions.md` | `Open Questions` | 未決事項があるときだけ | 機能の仕様でまだ決まっていない事項 |

- 用語集は、分類ごとに1ファイルにする（`spec/glossary/`）。1語ごとのファイルには分けない。用語集のディレクトリの concept は、`scope.md` も含めて type を `Glossary` にする。
- `spec/overview.md` は全体の要約で、1ファイルのままにする。

## ファイル名

- ファイル名とディレクトリ名は、英語の小文字・数字・ハイフンだけで書く。拡張子は `.md`。
- 内容を表す短い名前にする（例: `history.md`、`ai-input.md`）。
- 受け入れ条件は必ず `acceptance.md`、未決事項は必ず `open-questions.md` にする。
- ADR のファイル名は [ADR の運用ルール](decision-records.md) に従う。

## title

- その concept が何についての文書かを、単独で読んで分かる名前にする。
- 見出しが「画面」「概要」「流れ」のように短くなるときは、機能名を補う（例: 「質疑応答の画面」）。
- カリキュラムの章ごとのファイル（`spec/curriculum/chapters/` の `chNN-….md`）の title は、「章 ID + 章のタイトル」だけにする（例: `ch04 所有権`）。この title が章 ID と章のタイトルの定義を兼ねるので、語を足さない（[章とレッスンの一覧の読み方](../spec/curriculum/chapters/how-to-read.md)）。
- ADR の title は [ADR の運用ルール](decision-records.md) に従う。

## description

- その concept が何を定めているかを1文で書く。本文に書いてある範囲だけを使い、本文にない事実を書かない。
- 「〜を定める。」の形にそろえる。
- `Open Questions` だけは「〈機能名〉の仕様でまだ決まっていない事項（項目の要約）を記す。」の形にする。
- エージェントはこの1文を読んで、ファイルを開くかどうかを決める。中に何が書いてあるかが分かるよう、具体的な語（ファイル名、画面名、項目名）を入れる。
- ADR の description は [ADR の運用ルール](decision-records.md) に従う。
- title と description は平易な日本語で書く。体言止めや記号の多用を避ける。

例:

```markdown
title: "質疑応答の会話履歴"
description: "質疑応答の会話履歴を保持する範囲と、AI への入力に含める往復数を定める。"
```

## 本文

- 本文に `#` の見出しを書かない。文書の見出しは `title` が担う。
- 本文中の見出しは `##` から始める。
- 見出しに番号（`3.1` など）を付けない。
