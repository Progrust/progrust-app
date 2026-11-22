---
type: Decision
title: "ADR 0153"
description: "アプリは、リポジトリの `design-system/` にあるデザインシステムの CSS とアイコンを、写しを作らずに直接読み込み、書体を外部から読み込む `@import` の行はビルドのときに取り除き、`design-system/` を lint と整形の対象から外す"
---

- 日付: 2026-11-22
- 関連仕様: [画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステムの CSS とアイコン」、[リポジトリ構成](../spec/architecture/repository.md)、[開発環境と依存の版](../spec/architecture/development-environment.md)「lint と整形」、[画面仕様の未決事項](../spec/screens/open-questions.md)
- 置き換え: [ADR 0151](0151-copy-design-system-css-into-repository.md)

## 背景

[ADR 0151](0151-copy-design-system-css-into-repository.md) は、デザインシステムの CSS（`tokens.css`、`components/bundle.css`）とアイコンを `src/design-system/` に写して使うと決めた。写しは編集せず、書体を外部から読み込む `@import` の行だけを写しから取り除き、写しを取ったときの版と日付を記録し、README は写さないとした。写しを置くとしたのは、デザインシステムがリポジトリの外にあり、アプリがビルドに含めたファイルだけを読み込む（[ADR 0111](0111-bundle-monaco-and-set-csp.md)）からだった。

その後、デザインシステムのファイルをリポジトリの `design-system/` に置き、リポジトリのファイルを正とすると決めた（[ADR 0152](0152-keep-design-system-files-in-repository.md)）。デザインシステムのファイルは、アーティファクトへそのまま送る。

「Progrust」「Progrust Video」のどちらの `components/bundle.css` も、先頭の行が、書体を Google Fonts から読み込む `@import` である。この行は、アーティファクトのページの見本が書体を読み込むのに使っている。「Progrust」の README「実装のしかた」には「`bundle.css` は見本のために Murecho と Red Hat Mono を Google Fonts から読み込んでいる」とあり、「Progrust Video」の README にも、見本の `bundle.css` が Google Fonts から読み込んでいるとある。

この判断は、アプリの操作感に関わらない技術的な判断で、`CLAUDE.md` はそのような判断を Claude に任せている。Claude は、ADR 0151 を置き換えることを、作業の途中と、作業を終えたときの報告でユーザーに伝えた。ユーザーは、その報告に「コミットとプッシュお願いします。」と答えた。以下の決定と理由は、Claude の判断である。

## 決定

- アプリは、`design-system/progrust/`・`design-system/progrust-video/` の CSS（`tokens.css`、`components/bundle.css`）とアイコン（「Progrust」の `assets/Icons/`）を、`src/` の下に写しを作らずに、直接読み込む。`src/design-system/` は作らない（[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステムの CSS とアイコン」、[リポジトリ構成](../spec/architecture/repository.md)）。
  - 理由: ADR 0151 が写しを置くとしたのは、デザインシステムがリポジトリの外にあり、アプリがビルドに含めたファイルだけを読み込むからだった。デザインシステムがリポジトリに入ったので（ADR 0152）、同じリポジトリの中に同じファイルを 2 つ持つ理由が無くなった。写しを持つと、取り直しの作業と、取り直し忘れによる食い違いが残る。
- 書体を外部から読み込む `@import` の行（`components/bundle.css` の先頭）は、デザインシステムのファイルからは取り除かず、アプリがビルドのときに取り除いて読み込む。
  - 理由: アプリは外部のサイトから読み込まない（ADR 0111）。書体は同梱したファイルから読む（[ADR 0116](0116-bundle-upstream-variable-fonts-unmodified.md)）。一方、この行は、アーティファクトのページの見本が書体を読み込むのに使っている（「背景」を参照）。
  - 取り除く方法（ビルドの仕組みのどこで行うか）は、実装のときに決める。それまでは、未決事項として残す（[画面仕様の未決事項](../spec/screens/open-questions.md)）。
- アプリの画面のための CSS と、動画の共通部品のための CSS は、`design-system/` の中には書かず、`src/` の下に書く。ADR 0151 の「アプリのための CSS は、写しとは別のファイルに書く」を引き継ぐ。
  - 理由: デザインシステムのファイルは、アーティファクトへそのまま送る。アプリだけのための CSS を混ぜない。
- `design-system/` は、lint と整形の対象から外す（[開発環境と依存の版](../spec/architecture/development-environment.md)「lint と整形」）。[ADR 0148](0148-pin-dependencies-to-verified-versions-and-app-identifier.md) の「影響」が写しについて書いていたことの対象が、写しから `design-system/` に変わる。
  - 理由: デザインシステムのファイルは、アプリのコードの決まりではなく、Design System の種類が決める形に従う。`tokens.css` は `tokens.json` から作るファイルで、整形すると、作り直した内容と食い違う。
- ADR 0151 の「写しを取ったときのデザインシステムの版と日付を、写しと同じディレクトリに記録する」「デザインシステムの README は写さない」は、写しが無くなったので、無くなる。

## 検討した他の案

- 案: `src/design-system/` に写しを置き続ける（写す元を、リポジトリの `design-system/` にする）
  - 採用しなかった理由: 同じリポジトリの中に同じファイルを 2 つ持つことになり、取り直しの作業と、取り直し忘れによる食い違いが残る（「決定」を参照）
- 案: デザインシステムの `bundle.css` から `@import` の行を取り除く
  - 採用しなかった理由: アーティファクトのページの見本が、その行で書体を読み込んでいる

## 影響

- ADR 0151 の「影響」にあった、写しを取り直す作業と、取り直すたびに `@import` の行を取り除く作業は無くなる。代わりに、ビルドのときに `@import` の行を取り除く仕組みが要る。
- 部品の使い方は、リポジトリの `design-system/` の README で確かめられる。ADR 0151 の「影響」の「部品の使い方を確かめるには、リポジトリの外のデザインシステムを読む」は、当てはまらなくなった。
