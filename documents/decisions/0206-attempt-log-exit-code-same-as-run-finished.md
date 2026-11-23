---
type: Decision
title: "ADR 0206"
description: "試行ログの `run` の `exit_code` を、`run_finished` の `run.exit_code` と同じ値（学習者のプログラム、テスト実行では `cargo test` の終了コード。起動していないときとシグナルで終わったときは `null`）にする"
---

- 日付: 2026-11-23
- 関連仕様: [試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)、[実行のコマンドとイベント](../spec/architecture/commands/run.md)「イベント」
- 置き換え: なし

## 背景

試行ログの `run` イベントの `exit_code` が何の終了コードか（cargo か、学習者のプログラムか）と、シグナルで終わったとき・止めたときの値が、未決事項として残っていた。試行ログの例は `compile_error` に `exit_code: 1` を書いていたが、`run_finished` の `run.exit_code` は、プログラムを起動していないときは `null` としていて、食い違っていた。実装計画の M2 で、試行ログを書くために決める必要があった。

## 決定

- 試行ログの `run` の `result.exit_code` を、`run_finished` の `run.exit_code` と同じ値にする。学習者のプログラム（テスト実行では `cargo test`）の終了コードとし、プログラムを起動していないとき（`compile_error` と、ビルド中に止めた `stopped`）と、シグナルで終わったとき（停止・時間切れ・出力量の上限で止めたときを含む）は `null` にする。試行ログの例の `compile_error` の `exit_code` も `null` に直した。Claude が決めた。
  - 理由: 画面に送る値と記録する値を同じにすれば、1 つの値を作って両方に使える。学習記録が見たいのは学習者のプログラムの終わり方で、ビルドの失敗は `status` と `error_codes` で分かる。止めたときはプロセスに SIGKILL を送るので、終了コードが無い。

## 検討した他の案

- 案: `compile_error` のときに、ビルドの cargo の終了コードを入れる
  - 採用しなかった理由: 学習者のプログラムの終了コードと、同じ項目で意味が変わる。`run_finished` の値とも食い違う

## 影響

- 試行ログの `run` の `exit_code` は、`compile_error` のときに必ず `null` になる。
