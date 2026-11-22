---
type: Decision
title: "ADR 0166"
description: "表にない依存は、Node.js の系列（25.9 系）を `engines` が含み、表の依存の版と peer 依存が合う版を入れ、そのために Vitest を 4 系、jsdom を 29 系、`@testing-library/jest-dom` を 6 系、`stylelint-config-standard` を 40 系にする"
---

- 日付: 2026-11-22
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)「主な依存の版」
- 置き換え: なし

## 背景

起動できる空のアプリと開発で使う npm のコマンドを作るタスク（Issue #73）で、表（[開発環境と依存の版](../spec/architecture/development-environment.md)「主な依存の版」）にない道具（Vitest、jsdom、React Testing Library、Prettier など）を入れた。表にない依存は、入れたときの版をロックファイルで固定すると定めてあり、どの版を選ぶかは定めが無かった。

2026-11-22 に `npm view` で確かめたところ、いちばん新しい版のいくつかは、仕様の Node.js の系列（25.9 系）や、表の依存の版と合わなかった。

- Vitest 5.1.1 の `engines` は `^22.12.0 || ^24.0.0 || >=26.0.0` で、Node.js 25 を含まない。`npm install` は `EBADENGINE` の警告を出した。Vitest 4.1.12 の `engines` は `^20.0.0 || ^22.0.0 || >=24.0.0` で、Node.js 25 を含む。
- jsdom 31.0.1 の `engines` は `^22.22.2 || ^24.15.0 || >=26.0.0` で、Node.js 25 を含まない。jsdom 29.1.1 の `engines` は `^20.19.0 || ^22.13.0 || >=24.0.0` である。
- `@testing-library/jest-dom` 8.0.0 は、peer 依存に Vitest 5 系を求める。6.10.0 は Vitest 0.32 以上を求める。
- `stylelint-config-standard` 41 系は、peer 依存に stylelint 18 系を求める。表の stylelint は 17.16 系で、40 系が stylelint 17 系を求める。

ユーザーは、アプリの操作感に関わらない技術的に詳細な部分の判断を Claude に任せている（`CLAUDE.md`）。以下の決定と理由は、Claude の判断である。

## 決定

- 表にない依存は、Node.js の系列を `engines` が含み、表の依存の版と peer 依存が合う版を入れる（[開発環境と依存の版](../spec/architecture/development-environment.md)「主な依存の版」）。
  - 理由: `engines` や peer 依存の範囲の外の版は、作者が動くと示していない組み合わせで、動いても、パッチの更新で壊れたときに直してもらえる見込みが無い。Node.js の系列と表の依存の版は、技術検証で動いた組み合わせとして決めてある（[ADR 0148](0148-pin-dependencies-to-verified-versions-and-app-identifier.md)）ので、表にない依存のほうを合わせる。
- このため、Vitest を 4 系、jsdom を 29 系、`@testing-library/jest-dom` を 6 系、`stylelint-config-standard` を 40 系にする。版はロックファイルで固定し、表には書かない。

## 検討した他の案

- 案: いちばん新しい版（Vitest 5 系など）を入れ、`EBADENGINE` の警告は無視する
  - 採用しなかった理由: `engines` の範囲の外の Node.js で動かすことになる（「決定」を参照）
- 案: Node.js を 26 系に上げ、いちばん新しい版を入れる
  - 採用しなかった理由: Node.js の系列は、技術検証で動いた版として決めてある（ADR 0148）。上げると、確かめ直す手間が要り、このタスクの範囲を超える

## 影響

- Node.js の系列や表の依存の系列を上げるときは、表にない依存も、合う版に上げられるかを確かめる。たとえば、Node.js を 26 系以上に上げれば、Vitest 5 系と jsdom 31 系の `engines` に入る。
- 表にない依存の版を上げるときは、`engines` と peer 依存の範囲を確かめる。
