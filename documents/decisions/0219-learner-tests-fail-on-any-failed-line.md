---
type: Decision
title: "ADR 0219"
description: "`test` 方式の学習者のテストの判定で、` ... FAILED` の行が 1 つでもあれば cargo の終了コードが 0 でも `pass` にせず、cargo を `CARGO_TERM_COLOR=never` を付けて起動する"
---

- 日付: 2026-11-24
- 関連仕様: [コードの提出（submit）](../spec/exercise-runner/submit.md)「学習者のテストの判定」
- 置き換え: なし

## 背景

学習者のテストの判定（[ADR 0113](0113-judge-learner-tests-by-count-and-pass.md)）の結果の表では、`pass` を「cargo が終了コード 0 で終わり、` ... ok` の行が `min_learner_tests` 件以上ある」、`fail` を「` ... FAILED` の行がある、または ` ... ok` の行が足りない」としていた。この 2 つの行は、cargo が終了コード 0 で終わり、` ... ok` の行が足りていて、` ... FAILED` の行もあるときに、どちらにも当てはまる。

学習者のテストの中で `process::exit(0)` を呼ぶと、そのテストの実行ファイルは終了コード 0 で終わり、cargo も終了コード 0 で終わる（ADR 0113「影響」）。同じ実行ファイルの別のテストが、それより前に ` ... FAILED` の行を出していると、この重なりが起きる。表を上の行から当てはめると `pass` になり、受け入れ条件（[コード実行・判定の受け入れ条件](../spec/exercise-runner/acceptance.md)）の「学習者が書いたテストがすべて通り」に反する。

また、学習者のテストを実行する cargo の起動に、色の指定が無かった。テスト実行（[コードの実行（run）](../spec/exercise-runner/run.md)「テスト実行」）では `CARGO_TERM_COLOR=always` を付けている。2026-11-24 に手元の cargo で、`CARGO_TERM_COLOR=always` を付けて標準出力をパイプで受けても、テストとドキュメントテストの結果の行（`test {名前} ... ok`）には色が付かず、色が付くのは cargo 自身の標準エラー出力（`Running ...` など）だけであることを確かめた。

学習者のテストの判定の部品（Issue #162）を実装するときに見つけた。結果の決め方と cargo の起動のしかたなので、Claude が技術的な判断として決めた。

## 決定

[コードの提出（submit）](../spec/exercise-runner/submit.md)「学習者のテストの判定」を、次のように変えた。

- `pass` の条件に、「` ... FAILED` の行が無い」ことを足した。` ... FAILED` の行が 1 つでもあれば、cargo の終了コードが 0 でも `fail` にする。
  - 理由: ADR 0113 は、学習者のテストが「すべて通り」、指定の件数以上あることを `pass` の条件にしている。` ... FAILED` の行は、失敗したテストがあることを示すので、終了コードより優先する。
- 学習者のテストを実行する cargo を、環境変数 `CARGO_TERM_COLOR=never` を付けて起動する。
  - 理由: `timeout`・`runtime_error` の `detail` は cargo の標準エラー出力の先頭で、アプリを起動した環境に `CARGO_TERM_COLOR=always` があると、ANSI エスケープが入る。`detail` の中身が、アプリを起動した環境で変わらないように、色の無い文字にそろえる。結果の行の読み方は、色の指定で変わらない。

## 検討した他の案

- 案: 表を上の行から当てはめ、終了コード 0 で ` ... ok` の行が足りていれば `pass` にする
  - 採用しなかった理由: `process::exit(0)` を呼ぶテストと、失敗したテストが同じ実行ファイルにあると、失敗したテストがあっても `pass` になる
- 案: テスト実行と同じく `CARGO_TERM_COLOR=always` を付ける、または色を指定しない
  - 採用しなかった理由: 提出ではプログラムの出力をコンソールに表示しない（[コードの提出（submit）](../spec/exercise-runner/submit.md)）ので、色を付ける利点が無い。指定しないと、`detail` の中身がアプリを起動した環境で変わる

## 影響

- ` ... FAILED` の行があれば、cargo の終了コードによらず、「自分で書いたテスト」のテストケースは `fail` になり、`detail` に失敗したテストの名前が入る。
- 学習者のテストの `timeout`・`runtime_error` の `detail` には、色の ANSI エスケープが入らない。
