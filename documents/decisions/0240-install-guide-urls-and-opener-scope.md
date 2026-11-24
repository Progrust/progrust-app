---
type: Decision
title: "ADR 0240"
description: "導入手順の URL を rustup（https://rustup.rs）と Claude Code（https://code.claude.com/docs/ja/setup）の 2 つに決め、フロントエンドが OS の既定ブラウザで開ける URL をこの 2 つだけに絞る"
---

- 日付: 2026-11-24
- 関連仕様: [外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「見つからない場合」、[Tauri の権限](../spec/architecture/tauri-permissions.md)
- 置き換え: なし

## 背景

仕様は、`cargo`・`claude` が見つからないときに導入手順へのリンクを表示するとし、フロントエンドに外部 URL を OS の既定ブラウザで開く権限を許可するとしていたが、どの URL を開くかと、開ける範囲を定めていなかった。M3 で初回起動画面と設定画面を作るので、決める必要があった。

## 決定

以下は、どれも Claude の技術的な判断である。

- 導入手順の URL は、`https://rustup.rs` と `https://code.claude.com/docs/ja/setup` にする（[外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「見つからない場合」）。2026-11-24 に、どちらも開けて、導入手順が書かれていることを確かめた。
  - 理由: どちらも配布元の公式のページで、Claude Code のページは日本語で読める。
- フロントエンドが開ける URL は、この 2 つだけに絞る（[Tauri の権限](../spec/architecture/tauri-permissions.md)）。
  - 理由: 画面に表示する教材の Markdown や AI の回答にリンクが含まれても、アプリから任意のページを開けないようにする。開く URL を足すときは、仕様の表に足す。
- `rustc` のバージョンが足りないときは、リンクではなく `rustup update stable` の手順を示す。

## 検討した他の案

- 案: `https` の URL をすべて開けるようにする
  - 採用しなかった理由: 上の理由。開く URL は 2 つしかない

## 影響

- 配布元のページの URL が変わったら、仕様の表と、権限の設定を直す。
