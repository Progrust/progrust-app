---
type: Decision
title: "ADR 0232"
description: "開発時の起動で Vite が前もってまとめる依存を探す入口（optimizeDeps.entries）を、アプリの index.html だけにし、React の重複を確かめる自動テストを、塊の数ではなく全部の塊の中の React の本体の数で数えるようにする"
---

- 日付: 2026-11-24
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)「開発で使うコマンド」
- 置き換え: なし

## 背景

Issue #206 と #223 を直した後、作者がアプリを確かめるために開発時の起動（`npm run dev`）をすると、画面に何も表示されなかった。ブラウザの console には、[ADR 0199](0199-put-react-in-one-prebundled-chunk-for-dev.md) のときと同じ、React の「Invalid hook call」と、`useState` を読めないエラーが出ていた。

Claude が調べたところ、Vite が前もってまとめた依存（`node_modules/.vite/deps/`）の React の塊に、React の本体が 2 つ入っていた。1 つはリポジトリの `node_modules` のもの、もう 1 つは、gitignore された一時作業用のディレクトリにあった、前の技術検証の試作の `node_modules` のものだった。

- Vite は、`optimizeDeps.entries` を指定しないと、リポジトリの中の HTML をすべて（`node_modules` と出力先を除いて）入口にして、まとめる依存を探す。試作のディレクトリには、自分の `node_modules` を使う HTML があった。
- この前に、作業の中で `npm ci` を実行して `node_modules` を入れ直し、前もってまとめた依存が作り直された。それまでは、試作を置く前に作ったまとめが残っていたと考えられる。
- ADR 0199 の自動テスト（`tools/vite-deps.test.ts`）は、React の本体を含む塊の数を数えていた。2 つの React が同じ塊に入ったので、テストは通っていた。

## 決定

以下は、どれも Claude の技術的な判断である。

- `vite.config.ts` の `optimizeDeps.entries` を、アプリの `index.html` だけにする。
  - 理由: アプリの画面の入口は `index.html` だけである。教材の動画のコードなど、画面のコードが読み込むものは、Vite が `index.html` からたどって見つける。gitignore されたディレクトリに何を置いても、開発時の起動が壊れないようにする。
- `tools/vite-deps.test.ts` は、React の本体を含む塊の数ではなく、まとめた全部の塊の中にある React の本体の数を数え、1 つであることを確かめる。
  - [ADR 0199](0199-put-react-in-one-prebundled-chunk-for-dev.md) の「決定」の「React の本体を含む塊が 1 つだけであることを確かめる」を、この点で変える。React を 1 つの塊に入れる指定は変えない。
  - 理由: 同じ塊に 2 つの React が入ったときも、気づけるようにする。`optimizeDeps.entries` の指定を外すと、試作のディレクトリがある作者の Mac では、このテストが失敗することを確かめた。

## 検討した他の案

- 案: 試作のディレクトリを消す（または、HTML を置かない決まりにする）
  - 採用しなかった理由: 一時作業用のディレクトリには、何を置いてもよい。置いたものによって開発時の起動が壊れるのは、設定の側の問題である
- 案: `server.watch.ignored` や `optimizeDeps.exclude` で、そのディレクトリを外す
  - 採用しなかった理由: `watch.ignored` は、まとめる依存を探す入口に効かない。`exclude` は依存の名前で外す指定で、同じ名前の `react` を場所で分けられない。入口を指定するほうが、意図をそのまま書ける

## 影響

- アプリに `index.html` 以外の画面の入口（別のウィンドウの HTML など）を足すときは、`optimizeDeps.entries` にも足す。足さないと、その入口だけが読む依存は、前もってまとめられず、開いたときにまとめ直しが起きる。
- CI など、試作のディレクトリが無い環境では、`optimizeDeps.entries` の指定を外しても `tools/vite-deps.test.ts` は失敗しない。
