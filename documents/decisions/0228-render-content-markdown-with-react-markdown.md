---
type: Decision
title: "ADR 0228"
description: "教材の Markdown（問題文など）を、react-markdown で HTML の文字列を経ずに React の要素にして表示し、生の HTML は捨て、リンクは文字だけを表示する"
---

- 日付: 2026-11-24
- 関連仕様: [アプリの技術構成](../spec/architecture/tech-stack.md)、[演習画面](../spec/screens/exercise-screen.md)
- 置き換え: なし

## 背景

演習画面の左の列には、教材の問題文（`get_lesson` の `problem_md`）を Markdown として表示する。実装計画の M2 で、演習画面の枠を作るタスクが、Markdown の表示に使うものを決めることにしていた。タスクでは、問題文の生の HTML を表示しないことが求められていた。

## 決定

- 教材の Markdown は、react-markdown で表示する（[アプリの技術構成](../spec/architecture/tech-stack.md)）。CommonMark の範囲だけを読み、表などの GFM の拡張は、いまは入れない。
  - 理由: Markdown を HTML の文字列にせず、React の要素として組み立てるので、`dangerouslySetInnerHTML` も、HTML を消毒するライブラリも要らない。CSP（[ADR 0111](0111-bundle-monaco-and-set-csp.md)）の `script-src 'self'` のままで動く。
  - 理由: 教材の問題文の見出し（[演習のファイル形式](../spec/content-format/exercise.md)）は、コードブロック・箇条書き・文中のコードで書けている。GFM の拡張が要る教材は、まだ無い。
- 生の HTML は、表示せずに捨てる（react-markdown の `skipHtml`）。文中のタグを捨てると、タグに挟まれた文字は、ただの文字として残る。
  - 理由: タグを文字のまま見せても学習者の役に立たない。問題文に HTML が混ざっても、画面が崩れず、スクリプトも動かない。
- リンクは、文字だけを表示する。
  - 理由: アプリの画面は 1 つの WebView なので、リンクを押すと、アプリの画面がリンク先のページに移ってしまう。外部のブラウザで開く仕組みは、いまの教材に要らない。
- コードブロックは、デザインシステムの CodeBlock（`.pg-code`）で表示する。本文は ProblemPanel（`.pg-prose`）に従う。
- どれも、Claude が技術的な判断で決めた。

## 検討した他の案

- 案: `marked` で HTML の文字列にし、`DOMPurify` で消毒してから `dangerouslySetInnerHTML` で表示する（どちらも Monaco Editor の間接の依存として入っている）
  - 採用しなかった理由: 消毒の設定を誤ると、生の HTML やスクリプトが画面に入る。React の要素にすれば、その心配が無い。間接の依存を直接使うには、どのみち `package.json` に足す必要がある。
- 案: 生の HTML を、タグの文字のまま表示する（react-markdown の既定）
  - 採用しなかった理由: 「生の HTML を表示しない」に合わない。学習者には意味の無い文字が見える。
- 案: Markdown を表示するコードを自分で書く
  - 採用しなかった理由: CommonMark の細かい決まり（入れ子の箇条書きや、コードブロックの中の記号など）を正しく扱うのに手間がかかる。

## 影響

- 依存に `react-markdown` が加わる（[開発環境と依存の版](../spec/architecture/development-environment.md)「主な依存の版」の表にない依存として、ロックファイルで版を固定する）。
- 解説（`explanation_md`）を表示するタスクは、同じ部品（`src/markdown/`）を使える。
- 教材で表などの GFM の拡張を使いたくなったときは、`remark-gfm` を足す判断を、新しい ADR で行う。
