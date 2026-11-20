---
type: Convention
title: "ADR の運用ルール"
description: "設計判断記録（ADR）に記録する判断、ファイル名と番号、frontmatter での状態（`status`・`superseded_by`）の表し方、本文の書き方とテンプレート、作成後に変えてよい範囲、置き換えと廃止、仕様の「未決事項」との関係、`decisions/index.md` の構成、古い ADR の本文に出る仕様書の名前の読み方を定める。"
---

- ADR は `documents/decisions/` に置く。仕様の「なぜ」（その仕様にした理由・変更した理由）を記録する。
- 仕様は現時点の事実を書き、ADR は判断の経緯を書く。仕様の内容は ADR に書き写さず、仕様を参照する。
- 1つの判断につき1ファイルとする。

## 書く対象

次のいずれかに当てはまる判断を記録する。文言の調整など、細かい判断は記録しない。

- 他の案を検討したうえで選んだ
- 理由が仕様から読み取れない
- 後から変えるのが大変
- 後から疑問に思われそう、または別の案が提案されそう

## ファイル

- ファイル名は `NNNN-英語の短い説明.md`（例: `0001-learners-solve-on-their-own.md`）とする。番号は 0001 からの連番で、再利用しない。
- 下のテンプレートをコピーして作る。
- 作成したら、`decisions/index.md` に1行追加する（下の「索引」）。

## frontmatter

- `type` は `Decision`。
- `title` は `"ADR NNNN"`（例: `"ADR 0005"`）。
- `description` は、決定内容を一文で書く（例: `"スライドを、使える部品を決めた HTML で書く"`）。
- ADR の状態は、`status` と `superseded_by` で表す。どちらも引用符で囲まない。

| 状態 | 意味 | frontmatter |
| --- | --- | --- |
| 採用 | 現在有効な判断 | `status` も `superseded_by` も書かない |
| 置き換え済み | 後の ADR で別の判断に変わった | `status: deprecated` と、新しい ADR を指す `superseded_by: /decisions/NNNN-….md` |
| 廃止 | 判断の対象そのものがなくなった | `status: deprecated` だけ |

- frontmatter の共通の書き方は [frontmatter の書き方](frontmatter.md) に従う。

## 本文

- 本文の先頭に「日付」「関連仕様」「置き換え」の3行を書く。
  - 関連仕様は、関係する仕様の concept へのリンクで書く。リンクの文字は、リンク先の concept の `title` にする。複数あるときは「、」で区切る。
  - 「置き換え」には、この ADR が置き換える古い ADR へのリンク（`[ADR 0005](0005-slides-in-restricted-html.md)`）を書く。無ければ「なし」と書く。
- 続けて「背景」「決定」「検討した他の案」「影響」の見出しを書く。「決定」には仕様の詳細を書き写さず、関連仕様を参照する。
- 仕様と ADR のリンクの向きは [参照とリンクの書き方](links.md) に従う。

## テンプレート

````markdown
---
type: Decision
title: "ADR NNNN"
description: "（決定内容を一文で）"
---

- 日付: YYYY-MM-DD
- 関連仕様: [（リンク先の concept の title）](../spec/…)
- 置き換え: なし

## 背景

（どんな問題・制約があって、判断が必要になったか）

## 決定

（何を決めたか。仕様の詳細は書き写さず、関連仕様を参照する）

## 検討した他の案

- 案: （内容）
  - 採用しなかった理由: （理由）

## 影響

（この判断で得たもの・諦めたもの・今後の制約）
````

## 変更

- 採用した ADR の「背景」「決定」「検討した他の案」は書き換えない。誤字の修正や、関連仕様のリンク先の更新は行ってよい。
- 判断を変えたときは、新しい ADR を作り、「置き換え」に古い ADR へのリンクを書く。古い ADR の frontmatter に `status: deprecated` と、新しい ADR を指す `superseded_by` を書き（置き換え済み）、索引も更新する。
- 判断の対象そのものがなくなったときは、古い ADR の frontmatter に `status: deprecated` だけを書き（廃止）、索引も更新する。

## 仕様の「未決事項」との関係

- 仕様の「未決事項」（`open-questions.md`）が決まったときは、その判断を ADR として記録する。
- 決まった項目は、仕様に反映して `open-questions.md` から消す。項目が無くなったら `open-questions.md` を消し、そのディレクトリの `index.md` を直す。

## 索引

`decisions/index.md` は次のように書く。

- 採用の ADR は、関連仕様の先頭のリンク先が属する仕様ごとに、見出しで区切って載せる。見出しの名前は、その仕様が `spec/index.md` に載っている名前にする（例: `# 質疑応答`）。関連仕様の先頭が運用ルール（`conventions/`）の ADR は、見出し `# ドキュメントの運用ルール` の下に載せる。
- 置き換え済み・廃止の ADR は、末尾の見出し `# 置き換え済み・廃止` の下にまとめる。
- 各見出しの下では、番号順に並べる。
- 各行は `* [ADR NNNN](NNNN-….md) - description` の形にする。行の書き方の共通の決まりは [index.md の書き方と更新](index-files.md) に従う。

## 古い ADR の本文に出る名前

- 古い ADR（0001〜0078）の本文に出る `` `qa-chat.md` `` のような名前は、移行前の仕様書のファイル名で、いまは `documents/spec/` の同じ名前のディレクトリ（`` `qa-chat.md` `` なら `spec/qa-chat/`）に当たる。`overview.md` は `spec/overview.md` のまま。
