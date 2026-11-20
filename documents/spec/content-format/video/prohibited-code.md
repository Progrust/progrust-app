---
type: Specification
title: "動画のコードの禁止事項"
description: "教材チェックツールの lint でエラーにする動画のコード（色の直接指定、Tauri の API・ネットワーク・ファイルへのアクセス）を定める。"
---

以下を含む動画のコード（`video/` の `.tsx`・`.ts`・`.css`）は、教材チェックツール（[教材チェックツール](../../content-guidelines/check-tool.md)）の lint でエラーとする。

- 色の直接指定: 16 進表記（`#fff` など）、色の関数（`rgb()`・`hsl()`・`oklch()` など）、色の名前（`red` など）で色を書くこと。CSS の宣言、`style` に渡す値、SVG の色の属性（`fill`・`stroke` など）のどこに書いても対象とする。`none`・`currentColor`・`transparent`・`inherit` は書いてよい。
- Tauri の API へのアクセス: `@tauri-apps/` で始まるモジュールの import、`window.__TAURI__` などバックエンドを呼び出す手段の使用。
- ネットワークへのアクセス: `fetch`・`XMLHttpRequest`・`WebSocket`・`EventSource`・`navigator.sendBeacon` の使用、外部の URL（`http:`・`https:`・`//` で始まるもの）の参照（import、画像などの `src`、CSS の `url()`・`@import`）。
- ファイルへのアクセス: Node.js の組み込みモジュール（`fs` など）の import、`file:` の URL の参照。

これ以外（部品の作り方、動き、レイアウト）は、lint で制限しない。
