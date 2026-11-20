---
type: Decision
title: "ADR 0089"
description: "技術検証の結果を `documents/verification/` に1つの Issue につき1文書で残し、そのための type `Verification Report` を加える"
---

- 日付: 2026-11-20
- 関連仕様: [技術検証の結果の書き方](../conventions/verification-reports.md)、[ドキュメントの置き場所と構成](../conventions/bundle-structure.md)、[frontmatter の書き方](../conventions/frontmatter.md)「type の語彙」、[参照とリンクの書き方](../conventions/links.md)「仕様と ADR のリンクの向き」、[文書の分割単位と名前の付け方](../conventions/splitting.md)「description」、[ドキュメントの検査](../conventions/checks.md)
- 置き換え: なし

## 背景

実装を始める前に行う技術検証を、GitHub の Issue（ラベル `技術検証`）で管理している。ユーザーは、AI エージェントに検証を任せるときに、「検証した結果とくに問題ないのであればドキュメントに検証結果を記載してIssueは閉じて大丈夫です。」と示した。

それまでの `documents/` には、仕様（`spec/`）、ADR（`decisions/`）、運用ルール（`conventions/`）の置き場所しかなかった。仕様は現時点の事実を書くもので、検証した時点の観測（使った版、方法、出力）を書く場所ではない。ADR は判断を書くもので、問題がなかった検証には判断がない。`type` の6つの語彙（0080、0088）にも、検証の結果に合うものがなかった。

Claude が置き場所の案として「`documents/verification/` を新しく作り、1つの Issue につき1文書にする（検証結果を残して、後から引けるようにするため）」と「分かった事実を関係する仕様に反映し、測った値などの詳細は Issue のコメントに書く」を示し、ユーザーは前者を選んだ。

## 決定

- 技術検証の結果を、`documents/verification/` に、1つの技術検証の Issue につき1文書で置く。書き方は [技術検証の結果の書き方](../conventions/verification-reports.md) に定める。ユーザーの選択による。
- バンドルのルートのディレクトリを、`spec/`・`decisions/`・`conventions/`・`verification/` の4つにする（[ドキュメントの置き場所と構成](../conventions/bundle-structure.md)）。0088 で「ルートを3つのディレクトリにする構成」を保つために採らなかった変更を、ここで行う。
- 次の細部は Claude が決めた。
  - `type` の語彙に `Verification Report` を加えて7つにする。0080 の「type を6つに固定する」を、7つに変える。理由: 6つの語彙のどれにも、検証した時点の観測を書く文書は当てはまらない。`Specification` にすると、仕様（現時点の事実）と混ざる。
  - ファイル名を `issue-<Issue の番号>-<英語の短い説明>.md` にする。理由: Issue と文書の対応が、名前だけで分かる。
  - 試作のコードはリポジトリに残さず（一時作業用ディレクトリに置く）、判断の決め手になったコード・コマンド・出力を本文に書き写す。理由: 一時作業用ディレクトリは `documents/` から参照できない。本文だけで検証の根拠が分かるようにする。
  - 仕様から技術検証の結果へはリンクせず、技術検証の結果から仕様・ADR へ、ADR から技術検証の結果へリンクする。理由: 仕様は現時点の事実だけを書き、経緯は ADR が持つ。0082 の仕様と ADR のリンクの向きと同じ考え方にする。
  - 検証し直したときは同じ文書を書き換える。理由: 1つの Issue に1文書にしておけば、最新の結果だけを読めばよい。古い結果は git の履歴に残る。

## 検討した他の案

- 案: 分かった事実を関係する仕様に反映し、測った値などの詳細は Issue のコメントに書く
  - 採用しなかった理由: ユーザーが `documents/verification/` を新しく作る案を選んだ。
- 案: 検証の結果を `Specification` として `spec/` の各機能のディレクトリに置く
  - 採用しなかった理由: 仕様（現時点の事実）と、検証した時点の観測が混ざる。

## 影響

- 技術検証の Issue を閉じるときは、結果を `documents/verification/` に書いてから閉じる。
- 検査スクリプト（`tools/check-documents.py`）の `type` の語彙と、コードブロック・インラインコードの中のパスの検査に、`Verification Report` と `verification/` を加えた。
- 検証の結果、仕様を変えるときは、これまでどおり仕様を直して ADR を追加する。
