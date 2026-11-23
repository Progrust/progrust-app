---
type: Decision
title: "ADR 0187"
description: "同梱する教材を、ビルドスクリプトがリポジトリの content/ から選んで src-tauri/gen/bundled/content/ に写し、Tauri の bundle.resources はそのディレクトリだけを {app_resources}/content/ に入れる"
---

- 日付: 2026-11-23
- 関連仕様: [データの保存場所](../spec/architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」、[リポジトリ構成](../spec/architecture/repository.md)
- 置き換え: なし

## 背景

アプリ同梱データの `content/` には、リポジトリの `content/` から、動画のコードと、動画が読み込む画像を除いて入れる（[データの保存場所](../spec/architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」）。同梱するファイルは、Tauri の設定 `bundle.resources` で指定する。

Issue #88 で同梱の設定を作るときに、tauri-utils 2.10.4 の `bundle.resources` の処理（`src/resources.rs`）を読み、次のことが分かった。

- ディレクトリの中の一部のファイルを除く指定が無い。ディレクトリを指定すると、その下のファイルをすべて入れる。
- 指定の形には、パスの一覧と、元のパスから入れる先への対応表の 2 つがある。一覧の形では、`../content/…` は `{app_resources}/_up_/content/…` に入る。対応表の形でワイルドカード（`*`）を使うと、一致したファイルを、ディレクトリの構成を捨てて、入れる先の直下にファイル名だけで並べる。どちらも、レッスンごとのディレクトリの構成を保ったまま `{app_resources}/content/` に入れられない。
- ワイルドカードに一致するファイルが 1 つも無いと、ビルドが失敗する。`content/lessons/` にレッスンが 1 つも無いときに、レッスンのファイルを指すワイルドカードでビルドが失敗する。

この判断は、Claude が Issue #88 の実装で、技術的な判断として決めた。以下の理由は Claude が書いた。

## 決定

- ビルドスクリプト（`src-tauri/build.rs`）が、リポジトリの `content/` から同梱するファイルだけを選び、`src-tauri/gen/bundled/content/` に写す。`bundle.resources` は、対応表の形で、そのディレクトリを `{app_resources}/content/` に入れる（[データの保存場所](../spec/architecture/data-locations.md)「アプリ同梱データ（`{app_resources}`）」）。
  - 理由: 「背景」のとおり、`bundle.resources` だけでは、除くファイルを除き、構成を保って入れることができない。写したディレクトリを丸ごと入れる指定なら、構成を保てる。レッスンが無くても `chapters.json` があるので、空の一致でビルドが失敗することもない。
- 写すのは、Tauri のビルドの処理（`tauri_build::build()`）より前にする。
  - 理由: Tauri のビルドの処理は、開発時の起動のために、`bundle.resources` のファイルを実行ファイルの隣に写す。`cargo test` と `cargo clippy` でも同じ処理が動くので、写したディレクトリが無いと、CI のチェックが失敗する。ビルドスクリプトで写せば、`npm run dev`・`npm run build`・`npm run check` のどれでも、先に用意できる。
- 写したディレクトリは、git に入れない `src-tauri/gen/` の下に置く。中身が同じファイルは書き直さず、元から消えたファイルは写した先からも消す。
  - 理由: 写したものは、原本から作り直せる。書き直すとファイルの更新時刻が変わり、cargo がビルドスクリプトを実行し直す。
- 選ぶ規則（同梱するかどうか）は、ビルドスクリプトの部品（`src-tauri/build/`）に置き、テストからも同じ部品を使う。
  - 理由: 同梱するファイルと同梱しないファイルを、本番ビルドをしなくても、自動テストで確かめられる。

## 検討した他の案

- 案: `bundle.resources` に、同梱するファイルをワイルドカードで並べる（`../content/lessons/*/lesson.json` など）
  - 採用しなかった理由: 対応表の形ではディレクトリの構成が失われ、一覧の形では `_up_/` の下に入る。レッスンが無いと、ビルドが失敗する
- 案: `bundle.resources` に `content/` を丸ごと入れる
  - 採用しなかった理由: 動画のコードと画像も入り、仕様に合わない
- 案: npm のコマンド（`beforeBuildCommand` など）で写す
  - 採用しなかった理由: `cargo test`・`cargo clippy` だけを実行したとき（CI の `npm run check` を含む）に写されず、Tauri のビルドの処理が、写したディレクトリが無いために失敗する

## 影響

- `content/` を変えると、次のビルドでビルドスクリプトが実行し直され、写し直す。写したファイルが変わった次のビルドでは、もう一度ビルドスクリプトが実行される。
- 開発時の起動の最中に `content/` を変えても、自動では写し直さない。起動し直すと写し直す。
- 開発時の起動で、実行ファイルの隣に写したファイルは、`content/` から消しても消えない。写したディレクトリ（`src-tauri/gen/bundled/content/`）からは消える。
- 同梱するファイルの規則を変えるときは、ビルドスクリプトの部品とそのテストを直す。
