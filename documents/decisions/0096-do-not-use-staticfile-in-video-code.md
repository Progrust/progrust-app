---
type: Decision
title: "ADR 0096"
description: "動画のコードで Remotion の staticFile() を使わず、教材チェックツールの lint でエラーにする"
---

- 日付: 2026-11-21
- 関連仕様: [動画のコードの禁止事項](../spec/content-format/video/prohibited-code.md)
- 置き換え: なし

## 背景

画像は、レッスンの `assets/` か `content/assets/` に置いたファイルを import して使う（[動画のシーンの書き方](../spec/content-format/video/scenes.md)）。Remotion には、`public/` のファイルを指す `staticFile()` もある。

Issue #7 の技術検証で、同じシーンのコードを Remotion Studio・チェックツール・アプリの Vite ビルドの 3 か所で動かした（[動画の描画チェックを CI で実行できるかの技術検証](../verification/issue-7-render-check-in-ci.md)）。

- 画像の import は、3 か所とも動いた。
- `staticFile()` は、Remotion では `public/` から読めた。Vite では `/pub.png` のようなパスを返すだけで、Vite の `publicDir` を Remotion の `public/` に合わせないと画像を読めなかった。Tauri の本番ビルドでの配置は確かめていない。

チェックツールの描画は Remotion のバンドラーを使う。そのため、`staticFile()` で書いた画像がアプリで読めなくても、描画のチェックではエラーにならない見込みである（検証では確かめていない）。

Claude は、`staticFile()` を使わない決まりにすることを推奨した。ユーザーは、「このあたりの技術的な話は私はあまく詳しくないのであなたの推奨でお願いします。理由もあなたが書いてください。」と示し、判断と理由を Claude に任せた。以下の理由は Claude が書いた。

## 決定

- 動画のコードでは `staticFile()` を使わない。教材チェックツールの lint でエラーにする（[動画のコードの禁止事項](../spec/content-format/video/prohibited-code.md)）。
  - 理由: import なら 3 か所で同じように動く。`staticFile()` はアプリ側の設定に頼り、Tauri の本番ビルドでの配置も確かめていない。
  - lint で検査する理由: 使うとアプリの中でだけ画像が出なくなり、描画のチェックでは見つからない。import を検査するだけで見つけられる。

## 検討した他の案

- 案: Vite の `publicDir` を Remotion の `public/` に合わせて、`staticFile()` を使えるようにする
  - 採用しなかった理由: 画像の置き方が import と `public/` の 2 通りになる。Tauri の本番ビルドでの配置は確かめていない
- 案: 決まりにするが、lint では検査しない（ADR 0062 の lint は、色と、Tauri の API・ネットワーク・ファイルへのアクセスを禁じる最低限のものだったため）
  - 採用しなかった理由: 守られなかったとき、アプリで動画を見るまで気づけない

## 影響

- lint で禁じるものに、ADR 0062 の範囲（色と、アプリの権限を守るもの）の外の項目が 1 つ加わる。部品の作り方、動き、レイアウトは、引き続き lint で制限しない。
- `public/` に置くファイルは、動画のコードからは使わない。
- `staticFile()` を lint で検出することは、検証では試していない。lint を作るときに確かめる。
