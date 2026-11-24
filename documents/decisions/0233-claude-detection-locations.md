---
type: Decision
title: "ADR 0233"
description: "claude の検出で、npm のグローバルインストール先を npm を起動せずに /opt/homebrew/bin と /usr/local/bin とし、場所ごとの調べ方と使えない場所の飛ばし方を cargo と同じ形にする"
---

- 日付: 2026-11-24
- 関連仕様: [外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「検出」
- 置き換え: なし

## 背景

M3 で初回起動画面と設定画面の「環境」を作る。どちらも `claude` の検出結果を表示するので、`claude` の検出を M3 で作る。仕様は `claude` の探索順に「npm のグローバルインストール先」を挙げていたが、それがどこかと、場所ごとにどう調べるか（設定のパスの形、使えない場所の扱い、バージョンの取り出し方、時間切れ）を定めていなかった。

## 決定

以下は、どれも Claude の技術的な判断である。

- npm のグローバルインストール先は、`npm` を起動して調べず、`/opt/homebrew/bin` と `/usr/local/bin` を、この順に調べる（[外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「検出」）。
  - 理由: 対応環境は作者の Apple シリコンの Mac で（[ADR 0090](0090-support-only-authors-mac-until-completion.md)）、Node.js は Homebrew か公式のインストーラーで入れることが多い。`npm prefix -g` を起動すると、`npm` 自体を探す必要があり、起動も遅くなる。
- 場所ごとの調べ方、使えない場所を飛ばすこと、設定のパスを実行ファイルの絶対パスにすること、時間切れを 10 秒にすることは、`cargo` の決まり（[ADR 0210](0210-find-rustc-next-to-cargo-and-use-first-usable-place.md)）と同じ形にする。バージョンは、`claude --version` の出力の 1 行目の最初の語とする。
  - 理由: 2 つのツールの検出を同じ部品で作れ、学習者にも同じ形で説明できる。`claude` には必要なバージョンの条件が無いので、語の形は確かめない。

## 検討した他の案

- 案: `npm prefix -g` を起動して、グローバルインストール先を調べる
  - 採用しなかった理由: 上の理由。nvm などで入れた Node.js にも対応できるが、そのときは設定でパスを指定できる

## 影響

- nvm などで入れた Node.js の `claude` は、`PATH` に無いと（Finder から起動したときなど）見つからない。設定画面でパスを指定する。
