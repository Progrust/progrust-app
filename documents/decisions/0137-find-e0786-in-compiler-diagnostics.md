---
type: Decision
title: "ADR 0137"
description: "初回準備のやり直しの判定で、ビルド済みの依存の破損（E0786）を、cargo 自身の標準エラー出力ではなく、コンパイラの診断から探す"
---

- 日付: 2026-11-22
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備のやり直しが要る失敗」
- 置き換え: なし

## 背景

演習の `--offline` のビルドが終了コード 101 で失敗し、cargo 自身の標準エラー出力が、許可クレートのソースやビルド済みの依存が失われた 3 つの状態の文に当てはまるときは、初回準備のやり直しが要ると判定する（[ADR 0105](0105-detect-lost-crates-from-offline-build-errors.md)）。仕様も、3 つの状態のどれも、cargo 自身の標準エラー出力（`compiler-message` ではない行）から探すと書いていた。

元になった技術検証（[ビルドの時間と初回準備のやり直しの技術検証](../verification/issue-18-build-time.md)）は、`--message-format=json` を付けずに `cargo build --offline` を実行していた。このときは、コンパイラの診断も cargo の標準エラー出力に出る。

演習のビルドには `--message-format=json-diagnostic-rendered-ansi` を付ける（[コードの実行（run）](../spec/exercise-runner/run.md)「手順」）。Issue #54 の仕分けのとき（[ADR 0135](0135-show-cargo-stderr-with-quiet-build.md)「背景」）と Issue #55 の起票のときに、cargo 1.100.0・rustc 1.100.0（macOS）で、許可クレート rand の rlib を 0 バイトにして、`cargo build --offline --message-format=json-diagnostic-rendered-ansi` と `cargo test --offline --no-run --message-format=json-diagnostic-rendered-ansi` を試した。E0786 の診断（`note:` の rlib のパスを含む）は、標準出力の JSON の `compiler-message` にだけ出た。cargo 自身の標準エラー出力には、`error: could not compile ...` の行しか出なかった。`.crate` の欠落と、レジストリの索引の欠落の文は、cargo 自身の標準エラー出力に出た。

仕様のとおりに cargo 自身の標準エラー出力だけを見ると、ビルド済みの依存の破損を判定できない（Issue #55）。

また、このときの壊れた rlib のパスは `target/debug/build/rand/{ハッシュ}/out/librand-{ハッシュ}.rlib` で、技術検証の結果に書かれたパス（`target/debug/deps/librand-….rlib`）と違った。どちらもビルド先の中である。

Claude が決めた。以下の理由は Claude が書いた。

## 決定

- 判定に使う文を探す出力を、状態ごとに決める。`.crate` の欠落とレジストリの索引の欠落は、cargo 自身の標準エラー出力から探す。ビルド済みの依存の破損（E0786）は、コンパイラの診断（JSON の `compiler-message`）から探す（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備のやり直しが要る失敗」）。ADR 0105 の「決定」の「cargo 自身の標準エラー出力が、検証で確かめた 3 つの状態の文に当てはまるときは」を、この決まりに変える。
  - 理由: 実際のビルドと同じオプションで試すと、それぞれの文がその出力に出た。判定の条件（終了コード 101、2 つ以上の語の組、E0786 の `note:` のパスがビルド先の中を指すこと）は変えない。
- E0786 の `note:` のパスは、ビルド先の中であることだけを確かめ、ビルド先の中のどこを指すかは見ない。
  - 理由: 同じ壊し方でも、rlib のパスが技術検証のときと違った。ビルド先の中の置き場所は cargo の版で変わりうる。学習者のコードのエラーと取り違えないためには、ビルド先の中であることを確かめれば足りる。

## 検討した他の案

- 案: 演習のビルドに `--message-format=json` を付けるのをやめ、どの文も標準エラー出力から探す
  - 採用しなかった理由: コンパイラの診断を JSON で受け取り、`error_codes` を取り出す決まり（[実行・提出の試行ログへの記録](../spec/exercise-runner/attempt-log.md)）が成り立たなくなる
- 案: どの状態の文も、cargo 自身の標準エラー出力とコンパイラの診断の両方から探す
  - 採用しなかった理由: 文が出る場所は確かめてある。探す場所を広げると、学習者のプログラムのコードや文字列に同じ文があるときに取り違えるおそれが増える

## 影響

- ビルド済みの依存の破損を判定するには、コンパイラの診断を、ビルドが終わるまで受け取っておく必要がある。
- 技術検証の結果は、検証した時点の事実として書き換えない。
- 文が出る場所は cargo の版で変わりうる。確かめたのは cargo 1.100.0 だけである。
