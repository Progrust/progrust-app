---
type: Decision
title: "ADR 0195"
description: "実装のループは、始めるときに、始めた端末で見つかる node と npm の版が `package.json` の `engines` の系列に合うかを確かめ、合わないときは始めない"
---

- 日付: 2026-11-23
- 関連仕様: [実装のループ](../conventions/implementation-loop.md)「回す場所と始め方」
- 置き換え: なし

## 背景

Node.js は 25.9 系、npm は 11.12 系を使い、`package.json` の `engines` にも書く（[ADR 0148](0148-pin-dependencies-to-verified-versions-and-app-identifier.md)）。CI も Node.js 25.9 で動く。

実装のループは、始めた端末の環境のまま、`npm ci` と `npm run check` を実行する。Issue #122 は、ループの中で、Node.js v26.13.1・npm 11.20.0 が使われ、npm が `EBADENGINE` の警告を出したことを挙げていた。警告は出たが、ループは止まらずに進んだ。

2026-11-23 に、作者の Mac で次のことを確かめた。

- Node.js は、mise で 25.9.0 が入っている。作者の端末の設定では、mise の `node` が先に見つかり、25.9.0（npm 11.12.1）が使われる。
- Homebrew の Node.js v26.13.1 も入っていて、PATH に `/opt/homebrew/bin` がある。
- 2026-11-23 14:43 に始めたループの記録の `npm ci` の出力にも、`current: { node: 'v26.13.1', npm: '11.20.0' }` の警告があった。
- 作者の端末から、ループの作業ツリーをカレントディレクトリにして `node --version` を実行すると、25.9.0 だった。ディレクトリで版が変わるのではない。このことと記録から、ループを始めた端末では、Homebrew の `node` が先に見つかっていたと推測した。どの端末から始めたかは、確かめていない。

## 決定

- ループは、始めるときに、始めた端末で見つかる `node` と `npm` の版を、`package.json` の `engines` の系列（`~X.Y` の形のメジャーとマイナー）と比べる。合わないときは、ループを始めず、使われる `node`・`npm` の場所と、直し方を表示する。決まりは [実装のループ](../conventions/implementation-loop.md)「回す場所と始め方」に定める。Claude が決めた。
  - 理由: 系列の違う Node.js で、`package-lock.json` が書かれ、チェックが通ると、検証した版（ADR 0148）と違う版でマージされる。npm の警告だけでは、ループは止まらず、ユーザーも気づかない。どの `node` が見つかるかは、端末ごとの PATH で決まるので、リポジトリの側で版を変えることはできない。始める前に比べれば、ユーザーが、系列の合う端末から始め直せる。
  - `package.json` が無いときと、`engines` が `~X.Y` の形でないときは、比べない。

## 検討した他の案

- 案: npm の設定 `engine-strict` を、リポジトリの `.npmrc` で有効にする。
  - 採用しなかった理由: ループは、`package-lock.json` が変わったときだけ `npm ci` を実行するので、版の違いに気づくのが遅れる。`npm run check` は `engines` を見ない。作者が手で `npm install` するときにも止まるようになり、ループの外の作業に影響が広がる。
- 案: ループが、mise の `node` を探して使う。
  - 採用しなかった理由: 作者の Mac の入れ方（mise）を、スクリプトが前提にすることになる。入れ方を変えたときに、スクリプトも直す必要がある。
- 案: 仕様の系列を、26 系に上げる。
  - 採用しなかった理由: 系列を上げるには、その版での技術検証が要る（ADR 0148）。25.9 系は作者の Mac に入っていて、端末の PATH を直せば使える。

## 影響

- ループを始めた端末で、系列の違う `node` か `npm` が先に見つかると、ループは始まらない。ユーザーは、mise の `node` が先に見つかる端末から始め直すか、使わない Node.js（Homebrew のものなど）を外す。
- Node.js の系列を上げるときは、これまでどおり、仕様の表と `package.json` の `engines` を同じ変更で直す（[開発環境と依存の版](../spec/architecture/development-environment.md)「版を上げるとき」）。ループは、新しい `engines` の系列で比べる。
