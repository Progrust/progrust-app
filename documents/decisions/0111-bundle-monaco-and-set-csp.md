---
type: Decision
title: "ADR 0111"
description: "Monaco Editor を CDN を使わずに同梱し、アプリに CSP を設定して読み込み先を自身と Tauri の IPC に限り（Monaco のため style-src に 'unsafe-inline' を許す）、演習画面のショートカットを Monaco のコマンドとキャプチャの keydown で受ける"
---

- 日付: 2026-11-21
- 関連仕様: [アプリの技術構成](../spec/architecture/tech-stack.md)、[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「アプリ自体」、[演習画面](../spec/screens/exercise-screen.md)
- 置き換え: なし

## 背景

コードエディタには Monaco Editor を使い、補完・LSP 連携は行わない（[ADR 0004](0004-monaco-without-completion.md)）。Monaco を Tauri の WKWebView の中で、オフラインで、日本語入力と演習画面のショートカットとともに使えるかは確かめていなかった。仕様には CSP（Content Security Policy）の定めがなかった。

Issue #21 の技術検証で、次のことが分かった（[Monaco Editor を Tauri の中で使えるかの技術検証](../verification/issue-21-monaco-in-tauri.md)）。

- `@monaco-editor/react` は、既定では CDN から Monaco を読む。ローダーに同梱の `monaco-editor` を渡すと CDN を読まず、Web Worker も同梱のファイルで動いた。
- 読み込み先を自身と Tauri の IPC に限る CSP と両立した。ただし Monaco は実行時に `<style>` 要素と `style` 属性を作るため、`style-src` に `'unsafe-inline'` が要る（外すと違反が 96 件出た）。`script-src` は `'self'` のままでよく、`'unsafe-eval'` も要らなかった。
- Cmd+Enter・Cmd+Shift+Enter・Cmd+. は、Monaco の既定（行の挿入など）と重なるが、Monaco の `addCommand` で上書きできた。Monaco が処理するキーは window のバブリングには届かないので、エディタの外も含めて受けるには、window の keydown のキャプチャを併用する。Tauri の既定のメニューとは重ならない。
- 人が試作のアプリを操作して、日本語の変換と確定、変換中の矢印・Enter・Cmd+Enter、変換候補の位置、実際のキーでの 3 つのショートカットを確かめ、すべて問題なかった（ユーザーの言葉で「全部大丈夫だった」）。

検証のあと、Claude は、CSP を仕様に定めること、Monaco を同梱すること、ショートカットの受け方を推奨した。ユーザーは「私は細かい技術的な話はわからないので、あなたの推奨通りで進めて大丈夫です」と示し、判断を Claude に任せた。以下の理由は Claude が書いた。

## 決定

- Monaco Editor は CDN を使わず、アプリのビルドに同梱する。Web Worker も同梱する（[アプリの技術構成](../spec/architecture/tech-stack.md)）。
  - 理由: アプリはオフラインで演習できる必要がある（[オフライン時の動作](../spec/non-functional/offline.md)）。外部への通信は Claude Code の呼び出しとクレートのダウンロードに限るとしている（[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「データの送信」）。
- アプリのページに CSP を設定する。スクリプト・Worker・接続の読み込み先は、アプリ自身と Tauri の IPC に限る。`style-src` には `'unsafe-inline'` を許す。フォントと画像は `data:` も許す（[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「アプリ自体」）。
  - 理由: Tauri は CSP を設定しないと、何も制限しない。CSP があれば、誤って外部のスクリプトを読んだり外部へ送ったりするコードが入っても、ページが拒否する。検証では、外部への fetch が CSP で拒否されることを確かめた。`'unsafe-inline'` は Monaco を使う限り外せない。スタイルだけに許し、スクリプトには許さないので、危険は小さい。
- 演習画面のショートカットは、Monaco の `addCommand` で登録して Monaco の既定を上書きする。エディタの外にフォーカスがあるときも効くよう、window の keydown のキャプチャでも受ける。ただし、質疑応答の入力欄にフォーカスがあるときは受けない。日本語の変換中は、アプリ側の keydown の処理を行わない（[演習画面](../spec/screens/exercise-screen.md)「キーボードショートカット」）。
  - 理由: `addCommand` を使わないと、Cmd+Enter などで Monaco が行を挿入してしまう。Monaco が処理したキーはバブリングでは window に届かない。質疑応答の入力欄では Cmd+Enter を送信に使っている（[質疑応答の画面](../spec/qa-chat/screen.md)）ので、そこで実行が始まらないようにする。macOS の WebKit は、日本語の変換を確定する Enter の keydown を変換の後に送ることがあり、変換中の判定をしないと誤作動しうる。

## 検討した他の案

- 案: CSP を設定しない
  - 採用しなかった理由: ページの読み込み先が何も制限されない。設定しても、検証した範囲では動作に支障がなかった
- 案: `style-src` から `'unsafe-inline'` を外す
  - 採用しなかった理由: Monaco の動作に必要なスタイルが拒否される
- 案: Monaco を CDN から読む（`@monaco-editor/react` の既定）
  - 採用しなかった理由: オフラインで動かない。外部への通信が増える

## 影響

- Monaco の版を上げるときは、CDN を読まないこと、Worker が動くこと、CSP の違反が出ないことを確かめる。
- CSP は、Monaco だけの試作で確かめた。動画の Player、レッスン専用の CSS、音声など、ほかの部分がこの CSP の下で動くかは確かめていない。実装で確かめ、読み込み先を加える必要があれば、仕様に書いて加える。
- 補完・LSP を使わない（ADR 0004）ので、Rust では Cmd+.（Monaco の既定ではクイックフィックス）が何にも使われておらず、上書きしても失うものはない。
