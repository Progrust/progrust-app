---
type: Decision
title: "ADR 0141"
description: "Remotion の Player の共有の audio 要素が使う無音の data: の音声を拒否しないよう、CSP の media-src に data: を加える"
---

- 日付: 2026-11-22
- 関連仕様: [セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「アプリ自体」
- 置き換え: なし

## 背景

CSP の音声の読み込み先（`media-src`）は、アプリ自身と Tauri のアセットプロトコルに限っていた（[ADR 0111](0111-bundle-monaco-and-set-csp.md)、[ADR 0128](0128-serve-narration-from-resources-via-asset-protocol.md)）。文の音声は、Remotion の `Html5Audio` で鳴らす（[ADR 0092](0092-play-narration-with-html5audio.md)）。

Issue #46 の技術検証で、次のことが分かった（[音声をアプリ同梱データから asset:// で読む再生の技術検証](../verification/issue-46-serve-narration-via-asset-protocol.md)「Remotion の共有の `<audio>` の `data:` が拒否される」）。

- この CSP の下で動画を再生すると、`media-src` の違反が `data:` に対して出続けた（計測 1 回の間に 74 件）。
- Remotion の Player は、`Html5Audio` のために共有の `<audio>` を作り、使っていない `<audio>` の `src` に、無音の音声（`data:audio/mp3;base64,…`）を入れる。これが拒否される。
- 文の音声を鳴らすときは `src` が `asset://` の URL に替わるので、再生には影響しなかった。
- `media-src` に `data:` を足したビルドでは、`media-src` の違反は 0 件になり、配信の回数・量、再生、レッスンの切り替えの結果は変わらなかった。

仕様には、この違反を許すのか、`data:` を足すのかの定めが無かった（Issue #59）。

この判断は、ユーザーが呼んだ Issue の仕分け（`/triage-issues`）で、Claude が決めた。CSP を仕様に定める判断は、ADR 0111 でユーザーが Claude に任せた。以下の理由は Claude が書いた。

## 決定

- CSP の `media-src` に `data:` を加える（[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「アプリ自体」）。ADR 0111 と ADR 0128 の決定（スクリプト・Worker・接続の読み込み先を自身と IPC に限ること、`media-src` にアセットプロトコルを加えること）は変えない。
  - 理由: 仕様が選んだ音声の部品（`Html5Audio`）が、動くために読み込むものだからである。`data:` の音声は、ページの中に書かれたデータで、外部から読み込まず、外部へ送ることもない。音声として扱われ、スクリプトとして実行されることはない。フォントと画像にも `data:` を許している。
  - 理由: 違反が出続けると、本当に直すべき違反が埋もれる。CSP の違反は本番ビルドで見て確かめる（[ADR 0140](0140-check-csp-in-production-build.md)）ので、いつも出る違反は無くしておく。
  - 理由: 足したビルドで、`media-src` の違反が無くなり、再生と配信の結果が変わらないことを確かめてある。

## 検討した他の案

- 案: 違反を許したままにする
  - 採用しなかった理由: 再生には影響しないが、再生のたびに違反が出続け、ほかの違反を見つけにくくなる。本番ビルドで CSP の違反が出ないという受け入れ条件（ADR 0140）も満たせない
- 案: Remotion の Player の設定を変えて、共有の `<audio>` に無音の `data:` を入れないようにする
  - 採用しなかった理由: 確かめていない。共有の `<audio>` の使い方が変わると、ユーザーが聞いて部品を決めたとき（ADR 0092）や、Issue #46 で人が聞いたときと、音声の鳴らし方が違うものになる

## 影響

- ページの中のコードは、`data:` の音声を鳴らせるようになる。動画のコードは教材としてアプリのビルドに含め、外部から読み込まないので、外から音声を差し込まれることはない。
- Remotion の版を上げるときは、本番ビルドで `media-src` の違反が出ないことを確かめる。Remotion が共有の `<audio>` に入れるものが変わったら、`data:` が要るかを見直す。
