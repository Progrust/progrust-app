---
type: Decision
title: "ADR 0129"
description: "後の ADR が採用した ADR の決定の一部だけを変えたときは、古い ADR を採用のままにし、description の末尾に、変えた ADR と変えた内容を示す注記を足す"
---

- 日付: 2026-11-21
- 関連仕様: [ADR の運用ルール](../conventions/decision-records.md)「変更」
- 置き換え: なし

## 背景

ADR の運用ルールの「変更」には、判断を変えたときの扱いとして、古い ADR の全体を置き換える方法（`status: deprecated` と `superseded_by`）だけが書かれていた。後の ADR が、古い ADR の「決定」の一部だけを変えたときの扱いは、決まっていなかった（Issue #30）。

実際には、一部だけを変えた ADR が既にあった。どれも、新しい ADR の「決定」に変えた文を書き、「置き換え」を「なし」とし、古い ADR は変えていなかった。そのため、古い ADR を読んでも、その一部が変わったことが分からなかった。

- 例: [ADR 0088](0088-agents-file-found-problems-as-issues.md) は、[ADR 0080](0080-use-minimal-okf-frontmatter.md) の「`Convention` は `documents/` の中の文書の書き方だけに使う」を変えた。[ADR 0122](0122-hold-label-for-deferred-issues.md) と [ADR 0123](0123-refile-closed-issue-when-situation-changed.md) は、どちらも ADR 0088 の決定の一部を変えた。

似た場合の決まりとして、後の ADR が判断を別の対象に引き継いだときは、引き継がれた ADR の `description` の末尾に注記を足す、という決まりがある（[ADR 0087](0087-note-carry-over-in-adr-description.md)）。

Claude は、ADR 0087 と同じ形の注記を足すことを推奨した。ユーザーは「その他の内容は推奨通りで問題ありません。」と示し、推奨を承認した。以下の理由と、注記の文言、対象の線引きは Claude が決めた。

## 決定

- 後の ADR が、採用した ADR の「決定」の一部だけを変えたときは、古い ADR を採用のままにし、古い ADR の `description` の末尾に「（ADR NNNN で〈変えた内容〉を変えた）」を足し、索引の行も直す。`description` のそれ以外の部分と本文は変えない（[ADR の運用ルール](../conventions/decision-records.md)「変更」）。
  - 理由: エージェントは、索引の行と `description` を読んで、ADR を開くかどうかを決める。そこに注記があれば、古い ADR を読む前に、一部が変わったことと、どの ADR を読めばよいかが分かる。ADR 0087 の引き継ぎの注記と同じ形なので、決まりが増えない。本文を書き換えない決まり（[ADR 0083](0083-express-adr-status-in-okf-frontmatter.md)）も保てる。
- 一部だけを変えたとするのは、古い ADR の「決定」に書かれた文と、いまの決まりが食い違うようになったときとする。古い ADR が決めていなかったことを、後の ADR が足しただけのときは、注記を足さない。
  - 理由: 条件を足したり、決めていなかった細部を決めたりした ADR は多い。それらにも注記を足すと、`description` が長くなり、本当に食い違うようになった ADR が埋もれる。
- 新しい ADR の「決定」には、どの ADR のどの文を、どう変えたかを書く。「置き換え」は「なし」とする。
  - 理由: それまでの書き方と同じである。「置き換え」は、古い ADR の全体を置き換えたことを表す行で、古い ADR の状態（`status: deprecated`）と対になっている。
- 既にあった次の 6 組について、古い ADR の `description` に注記を足した。
  - [ADR 0002](0002-target-experienced-programmers.md): [ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md) が、既知とする概念の範囲を変えた。
  - [ADR 0079](0079-manage-documents-as-okf-bundle.md): [ADR 0089](0089-keep-verification-reports-in-documents.md) が、バンドルのルートのディレクトリを 3 つから 4 つに変えた。
  - ADR 0080: ADR 0088 が `Convention` の範囲を、ADR 0089 が `type` の数を変えた。
  - ADR 0088: ADR 0122 がラベルの数を、ADR 0123 が閉じた Issue と同じ問題の扱いを変えた。
- 同じ日に追加した [ADR 0125](0125-crates-unavailable-status-without-attempt-log.md) が、[ADR 0029](0029-keep-full-attempt-logs.md) の決定の一部を変えたので、ADR 0029 にも注記を足した。

## 検討した他の案

- 案: frontmatter に、変えた ADR を指す新しいキーを足す
  - 採用しなかった理由: frontmatter のキーは 5 つだけにしている（ADR 0080）。検査の `frontmatter-keys` も、この 5 つだけを認める。キーを足すと、その両方を変えることになる
- 案: 古い ADR の本文に、変わったことを追記する
  - 採用しなかった理由: 採用した ADR の「背景」「決定」「検討した他の案」は書き換えない決まりである（ADR 0083）
- 案: 一部を変えるときも、古い ADR の全体を新しい ADR で置き換える
  - 採用しなかった理由: 一文を変えるだけで、古い ADR の変わらない判断も、すべて新しい ADR に書き直すことになる。ADR 0088 のように、ユーザーの言葉や多くの細部を持つ ADR では、書き直しが大きい
- 案: 何も足さず、新しい ADR の「決定」だけに書く（それまでの書き方）
  - 採用しなかった理由: 古い ADR を読んでも、一部が変わったことが分からない

## 影響

- 今後、古い ADR の決定の一部を変える ADR を書くときは、古い ADR の `description` と索引の行にも注記を足す必要がある。
- 注記の形と、足し忘れは、検査では確かめられない。
- 一部だけを変えたかどうかは、ADR を書く AI エージェントが読んで決めるので、決め方に幅がある。0001〜0123 を調べたとき、当てはまるかどうかを迷って、注記を足さなかった組がある。たとえば次の 2 組である。
  - [ADR 0060](0060-switch-lessons-from-slides-to-videos.md) と [ADR 0119](0119-video-only-lessons-for-background-knowledge.md): ADR 0060 の「レッスンは「動画 1 本 + 演習 1 問」になる」に当てはまらないレッスンが増えた。ただし、当てはまらないレッスン（章末復習レッスン）は、ADR 0119 より前からあった。
  - [ADR 0012](0012-separate-build-dir-without-crates.md) と [ADR 0098](0098-separate-build-dir-for-validation.md): 生成問題の検証のビルド先が変わった。ADR 0098 は、ADR 0012 は `project/` の中での分け方としてそのまま有効である、と書いている。
- 注記が増えると、`description` が長くなる。
