---
type: Decision
title: "ADR 0101"
description: "test 方式の隠しテストを、テストごとに別のプロセスで実行し、終了コードと結果の行で判定する"
---

- 日付: 2026-11-21
- 関連仕様: [コードの提出（submit）](../spec/exercise-runner/submit.md)「test 方式」
- 置き換え: なし

## 背景

これまでの仕様では、`test` 方式の判定で、テスト実行ファイルを `progrust_judge::` で絞り込んで 1 回だけ起動し、出力の `test progrust_judge::{関数名} ... ok|FAILED` の行を読んでいた。

Issue #15 の技術検証で、この方式（single）と、テストごとに別のプロセスで起動する方式（per-test）を比べた（[test 方式のテスト結果の読み取りの技術検証](../verification/issue-15-test-result-parsing.md)）。

- single でも、出力を捕捉する既定の形式なら、学習者の `println!` は結果の行と混ざらず、結果と panic メッセージを取り出せた。
- single では、テストの 1 件が `std::process::exit(0)` を呼ぶと、結果の行が欠けたまま終了コード 0 で終わり、全体が成功したように見えうる。abort やスタックオーバーフローでは全体が落ち、原因のテストが分からない。
- per-test（`--list` で名前を得て、名前ごとに `--exact` で別のプロセスを起動する）では、これらも含めてテストごとに `runtime_error`・`timeout` を付けられた。10 個のテストで、所要時間の差は 0.01 秒だった。
- `--nocapture` を付けると、学習者の出力が結果の行の途中に入り、結果の行が壊れる。
- `judge.target` が `src/main.rs`・`src/lib.rs` 以外のファイルだと、テストの名前に `geo::progrust_judge::t_ok` のようなモジュールの経路が付く（[隠しテストを読み込む #[path] の解決の技術検証](../verification/issue-14-hidden-test-path-resolution.md)）。

Claude は per-test を推奨した。ユーザーは、「私は細かい技術的な話はわからないので、あなたの推奨通りで進めて大丈夫です」と示し、判断を Claude に任せた。以下の理由は Claude が書いた。

## 決定

- 隠しテストの名前を `--list progrust_judge::` で得て、名前ごとに `--exact {名前} --test-threads=1` で別のプロセスとして起動する（[コードの提出（submit）](../spec/exercise-runner/submit.md)「test 方式」）。
  - 理由: 1 件のテストの `process::exit`・abort・スタックオーバーフロー・無限ループが、他のテストの結果に影響しない。判定結果の `cases[].status` の 4 種類（`pass`・`fail`・`timeout`・`runtime_error`）を、テストごとにそのまま付けられる。
- 結果は、終了コードと、そのプロセスの結果の行で決める。結果の行がないまま終了コード 0 で終わったテストは `runtime_error` にする。
  - 理由: 終了コードだけでは、`process::exit(0)` で終わったテストを成功と見誤る。
- テストケースの関数名は、`--list` で得た名前の `progrust_judge::` より後の部分とする。
  - 理由: 名前の前にモジュールの経路が付いても、`case_labels` のキー（テスト関数名）と対応させられる。
- `--nocapture` は付けない。
  - 理由: 結果の行が壊れる。

## 検討した他の案

- 案: 1 回だけ起動して出力を読む（これまでの仕様）
  - 採用しなかった理由: `process::exit(0)` で全体が成功したように見えうる。abort などでは原因のテストが分からず、他のテストの結果も失われる。`--list` の件数と結果の行の件数を照らし合わせれば見誤りは防げるが、テストごとの `runtime_error` は付けられない
- 案: `RUSTC_BOOTSTRAP` を付けて、libtest の JSON 形式の出力を使う
  - 採用しなかった理由: stable の rustc で nightly の機能を使う非公式の方法で、将来の rustc で動く保証がない

## 影響

- テストの数だけプロセスを起動するので、プロセスの管理と並列数の制御の実装が増える。
- 学習者が出力の捕捉を避けて標準出力に直接書いても、結果は終了コードで決まるので、偽の結果の行の影響を受けにくい。ただし、悪意を持って合否を偽るコードは防げない。
- 隠しテストを組み込む方式（[ADR 0009](0009-embed-hidden-tests-as-module.md)）と、`progrust_judge::` で絞り込むことは変わらない。
- 隠しテストが 1 件もないとき（`--list` が 0 件のとき）の扱いは、確かめていない。教材の隠しテストは 3 件以上用意する。ただし、ch01・ch02 のレッスン（章末復習レッスンを含む）の演習は 1 件以上でよい（[演習の作成ルール](../spec/content-guidelines/exercise.md)「テストケース」）。
