---
type: Decision
title: "ADR 0253"
description: "Dependabot alerts が報告した依存の脆弱性に対応して、monaco-editor を 0.58 系に上げて間接の依存の dompurify を直った版にし、許可クレートの rand を 0.9.5 に上げる"
---

- 日付: 2026-11-26
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)「主な依存の版」、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「許可クレート」
- 置き換え: なし

## 背景

リポジトリを公開して Dependabot alerts を有効にしたところ、依存の脆弱性が 6 件報告された（Issue #268）。重大度は、どれも low である。

- `dompurify` の 5 件（`IN_PLACE` のモード、`SAFE_FOR_TEMPLATES` などの特定の使い方での迂回）。直った版は 3.4.17 である。アプリは `dompurify` を直接使わない。`monaco-editor` 0.57.0 が、`dompurify` を 3.4.15 に厳密に固定して依存している。`package.json` の `monaco-editor` の範囲（`~0.57.0`）の中では、ロックファイルの更新だけで直った版にできない。`monaco-editor` 0.58.0 は、`dompurify` 3.4.17 に依存している。
- `rand` の 1 件（「Rand is unsound with a custom logger using rand::rng()」）。直った版は 0.9.3 である。許可クレートの一覧（`runner/allowed-crates.toml`）が `rand` を `=0.9.1` に固定している。

`monaco-editor` の系列は、技術検証で動いた系列にそろえている（[ADR 0148](0148-pin-dependencies-to-verified-versions-and-app-identifier.md)）。

Claude は、`dompurify` について、`monaco-editor` を 0.58 系に上げる、`package.json` の `overrides` で `dompurify` だけを差し替える、保留する、対応しない、の 4 つを示し、`monaco-editor` を 0.58 系に上げることを推奨した。ユーザーは、推奨のとおり、`monaco-editor` を 0.58 系に上げることを選んだ。`rand` は、版を上げて直すと伝えた。以下の理由は Claude が書いた。

## 決定

- `monaco-editor` を 0.58 系に上げる。`dompurify` は、`monaco-editor` 0.58.0 の依存のとおり、3.4.17 になる。系列の表は [開発環境と依存の版](../spec/architecture/development-environment.md)「主な依存の版」に定める。
  - 理由: `monaco-editor` の作者が組み合わせた版のまま、直った `dompurify` を使える。`overrides` で `dompurify` だけを差し替えると、`monaco-editor` が想定していない版の組み合わせになり、その組み合わせで動くことを、だれも確かめていない。
  - 技術検証で動いた系列から外れるので、[開発環境と依存の版](../spec/architecture/development-environment.md)「版を上げるとき」のとおり、本番ビルドで、演習画面のエディタ（日本語の入力、ショートカット）と、CDN を読まずに同梱の Web Worker で動くこと、CSP の違反が出ないことを、人が操作して確かめる。
- 許可クレートの `rand` を、`=0.9.1` から `=0.9.5` に上げる。一覧は [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「許可クレート」に定める。これは Claude の技術的な判断である。
  - 理由: 0.9.5 は、直った版（0.9.3）以降で、同じ 0.9 系の最新である。0.9 系の中の更新なので、教材と学習者のコードの書き方は変わらない。0.10 系は、API が変わりうるので選ばない。

## 検討した他の案

- 案: `package.json` の `overrides` で、`dompurify` だけを 3.4.17 に差し替える
  - 採用しなかった理由: 上の理由。
- 案: `dompurify` の 5 件は、アプリがその使い方に当たらないことを確かめて、アラートを閉じる
  - 採用しなかった理由: Monaco の内部での使い方を確かめる手間がかかり、Monaco の版を上げるたびに確かめ直す必要がある。直った版が出ているので、版を上げるほうが確かである。
- 案: `rand` を、直った最小の版の 0.9.3 にする
  - 採用しなかった理由: 0.9 系の中では、新しいパッチのほうが、ほかの修正も含む。

## 影響

- `monaco-editor` 0.58.0 は、既定の入口（`editor.main`）に、編集機能 `structuralDiff` を足した。[ADR 0225](0225-lean-monaco-entry-rust-tokens-and-theme-from-css-variables.md) のとおり、自前の入口（`src/editor/monaco.ts`）にも同じ順で足した。この機能が使う `@vscode/diff` は、新しい間接の依存になった。WebAssembly の版は、差分の方式を `advanced-wasm` にしたときだけ使われ、アプリのビルドには WebAssembly のファイルが出ない。
- 許可クレートの一覧と `runner/Cargo.lock` が変わるので、同梱のハッシュ値が変わり、学習者の環境では、次に起動したときに初回準備がやり直される（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)）。`runner/Cargo.lock` では、`rand` のほかに、間接の依存の `zerocopy`・`zerocopy-derive` が 0.8.64 から 0.8.65 になった。
- 本番ビルドでのエディタと CSP の確認は、ユーザーが行う。
