---
type: Decision
title: "ADR 0199"
description: "開発時の起動で Vite が前もってまとめる依存（optimizeDeps）の分け方を指定し、react・react-dom・scheduler を 1 つの塊に入れる"
---

- 日付: 2026-11-23
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)「開発で使うコマンド」
- 置き換え: なし

## 背景

見本の教材（ch04-01・ch04-04。Issue #112）を足した後、開発時の起動（`npm run dev`）で、アプリの画面に何も表示されなくなった。ブラウザの console には、React の「Invalid hook call」と、`useState` を読めないエラーが出ていた。

Claude が調べたところ、Vite 8.3.8（rolldown 1.2.19）が開発時に前もってまとめる依存（`node_modules/.vite/deps/`）で、React の本体（`react/cjs/react.development.js`）が 2 つの塊に重複して入っていた。画面のコードは片方の React を、react-dom と remotion はもう片方の React を使うので、フックが動かない。

- まとめる依存の一覧（`react`・`react-dom`・`remotion`・`@remotion/player` など）は、教材があるときもないときも同じだった。違うのは塊への分け方だけだった。
- 教材を足す前のコミット（3502ee5）では、まとめを作り直しても React は 1 つの塊だけに入り、画面は表示された。教材を足した後は、まとめを作り直しても毎回重複した。教材の動画のコードが読み込む `remotion` などを、Vite が依存を探すときに見つけることで、分け方が変わると考えられる。
- 本番ビルド（`vite build`）では、React は 1 つのファイルに入り、重複しなかった。前もってまとめる依存は、開発時の起動だけで使う。

## 決定

`vite.config.ts` の `optimizeDeps.rolldownOptions.output.codeSplitting.groups` で、`node_modules` の `react`・`react-dom`・`scheduler` を、名前 `react` の 1 つの塊に入れる。Claude が技術的な判断で決めた。

- 理由: 分け方を固定すれば、教材の動画のコードが増えても、React が重複しない。指定した後は、教材があるときも、React は 1 つの塊だけに入り、画面が表示された。
- 自動テスト（`tools/vite-deps.test.ts`）で、いまのリポジトリでまとめた依存のうち、React の本体を含む塊が 1 つだけであることを確かめる。指定を外すと、このテストは失敗する。

## 検討した他の案

- 案: 開発時の起動の前に、まとめを作り直す（`--force`、`node_modules/.vite` を消す）
  - 採用しなかった理由: 作り直しても、毎回重複した
- 案: `resolve.dedupe` に `react`・`react-dom` を書く
  - 採用しなかった理由: `dedupe` は、別の場所にある同じパッケージを 1 つの場所に解決する指定である。今回は、同じ場所の同じファイルが 2 つの塊に入っているので、効かない
- 案: Vite か rolldown の版を変えて、直るのを待つ
  - 採用しなかった理由: 依存の版は、技術検証で動いた系列にそろえている（[ADR 0148](0148-pin-dependencies-to-verified-versions-and-app-identifier.md)）。開発時の起動が使えないままになる

## 影響

- Vite か rolldown の版を上げて、この不具合が直ったと分かったら、この指定は外してよい。外す前に、`tools/vite-deps.test.ts` が通ることを確かめる。
- 本番ビルドの分け方は変わらない。
