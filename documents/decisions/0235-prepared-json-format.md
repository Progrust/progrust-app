---
type: Decision
title: "ADR 0235"
description: "prepared.json に schema_version を持たせ、crates_hash を同梱の allowed-crates.toml と Cargo.lock を続けたものの SHA-256 にし、読み込めない prepared.json は退避も通知もせずに無いものとして扱う"
---

- 日付: 2026-11-24
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」
- 置き換え: なし

## 背景

非機能要件の未決事項に、「`prepared.json` が `schema_version` を持つか」があった（[データの互換性](../spec/non-functional/data-compatibility.md) は、アプリが管理するファイルはすべて `schema_version` を持つとしている）。M3 で `prepared.json` を作るので、決める必要があった。あわせて、`crates_hash` の求め方と、読み込めないときの扱いも定めていなかった。

## 決定

以下は、どれも Claude の技術的な判断である。形は [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」の「`prepared.json`」にある。

- `prepared.json` は `schema_version`（`1`）を持つ。
  - 理由: データの互換性の決まりに合わせる。形を変えたときに、古い形を見分けられる。
- `crates_hash` は、`allowed-crates.toml` のバイト列に `Cargo.lock` のバイト列を続けたものの SHA-256 とする。
  - 理由: Rust の標準ライブラリのハッシュは、版によって値が変わりうると定められている。SHA-256 は広く使われ、値が変わらない。
- 読み込めない `prepared.json` は、無いものとして扱い（初回準備をやり直す）、退避も通知もしない。
  - 理由: `prepared.json` は、初回準備をやり直せば作り直せる、ビルドの状態の記録で、学習者のデータではない。`settings.json` などの退避と通知の決まり（[データの保存場所](../spec/architecture/data-locations.md)「書き込みの規則」）は、学習者が失うものを知らせるためにある。

## 検討した他の案

- 案: `crates_hash` を求めず、同梱のファイルの中身を `prepared.json` に写して比べる
  - 採用しなかった理由: `Cargo.lock` は長く、`prepared.json` を読むたびに大きな文字列を比べることになる
- 案: 読み込めない `prepared.json` を、`settings.json` と同じく退避する
  - 採用しなかった理由: 上の理由。退避したファイルを学習者が使う場面が無い

## 影響

- `prepared.json` の形を変えたときは、`schema_version` を上げる。古い版は読み込めないものとして扱われ、初回準備がやり直される。
