---
type: Specification
title: "動画のコードの禁止事項"
description: "教材チェックツールの lint でエラーにする動画のコード（色の直接指定、Tauri の API・ネットワーク・ファイルへのアクセス、CSS の `@import`、検査をすり抜ける書き方、`staticFile()`）と、lint が検出する範囲と、lint を止める指定（`eslint-disable` など）に従わないことを定める。"
---

以下を含む動画のコード（`video/` の `.tsx`・`.ts`・`.css`）は、教材チェックツール（[教材チェックツール](../../content-guidelines/check-tool.md)）の lint でエラーとする。`.tsx`・`.ts` は ESLint で、`.css` は stylelint で検査する。

- 色の直接指定: 16 進表記（`#fff` など）、色の関数（`rgb()`・`hsl()`・`oklch()` など）、色の名前（`red` など）で色を書くこと。`none`・`currentColor`・`transparent`・`inherit` は書いてよい。
  - 16 進表記と色の関数は、動画のコードのどの文字列に書いても対象とする。
  - 色の名前は、CSS の宣言、`style` のプロパティの値、SVG の色の属性（`fill`・`stroke` など）に、文字列で直接書いたものを対象とする。
- Tauri の API へのアクセス: `@tauri-apps/` で始まるモジュールの import、`__TAURI__`・`__TAURI_INTERNALS__`（`window.__TAURI__` など）の使用。
- ネットワークへのアクセス: `fetch`・`XMLHttpRequest`・`WebSocket`・`EventSource`・`navigator.sendBeacon` の使用、外部の URL（`http:`・`https:`・`//` で始まるもの）の参照（import、画像などの `src`、CSS の `url()`）。
- ファイルへのアクセス: Node.js の組み込みモジュール（`fs` など）の import と `require()`、`file:` の URL の参照。
- CSS の `@import`: 参照先によらず使わない。CSS は TSX から import する。
- 検査をすり抜ける書き方: `eval`・`new Function`、引数が文字列リテラルでない動的 import、`window`・`globalThis`・`self`・`navigator` への `[...]` でのアクセス、型アサーション（`(window as any)` など）を通した `window`・`globalThis`・`self`・`navigator` の使用。
- Remotion の `staticFile()` の使用（画像の使い方は [動画のシーンの書き方](scenes.md)）。

lint は、コードの中に書いた、lint を止めるための指定（`eslint-disable`・`stylelint-disable` などのコメント）に従わない。こうした指定があっても、違反はエラーになる。

lint は、書かれたとおりの文字列と名前を検査する。実行時に組み立てた文字列（`"r" + "ed"` など）や、変数・別名を経由したもの（`const w = window; w.fetch()` など）は検出しない。こうした書き方で禁止事項を避けてはならない。

これ以外（部品の作り方、動き、レイアウト）は、lint で制限しない。
