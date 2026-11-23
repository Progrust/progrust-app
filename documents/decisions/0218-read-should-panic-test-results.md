---
type: Decision
title: "ADR 0218"
description: "`test` 方式で、名前の後に ` - should panic` などのテストの種類の表示が付いた行も結果の行とし、`#[should_panic]` のテストの失敗では、libtest の説明を `fail` の `detail` に入れる"
---

- 日付: 2026-11-24
- 関連仕様: [コードの提出（submit）](../spec/exercise-runner/submit.md)「test 方式」、[レッスンの演習を作るときの注意](../spec/curriculum/exercise-constraints.md)
- 置き換え: なし

## 背景

`test` 方式の判定では、隠しテストをテストごとに別のプロセスで実行し、結果の行（` ... ok` / ` ... FAILED`）と終了コードで結果を決める（[ADR 0101](0101-run-each-hidden-test-in-own-process.md)）。これまでの仕様は、結果の行を「行末の ` ... ok` などの直前に `test {名前}` がある行」としていた。

ch09-01（panic）の演習では、隠しテストに `#[should_panic]` を使う（[レッスンの演習を作るときの注意](../spec/curriculum/exercise-constraints.md)）。rustc stable の libtest は、`#[should_panic]` のテストの結果の行を `test progrust_judge::x - should panic ... ok` の形で出す。テスト実行ファイルを `--exact {名前} --test-threads=1` で起動して確かめた。これまでの読み方では結果の行が見つからず、正しい答えも誤った答えも `runtime_error` になり、ch09-01 の演習を正解にできなかった。

あわせて、`#[should_panic]` のテストの失敗では、次のことも確かめた。

- panic しなかったときは、`panicked at` の行が無く、`failures:` の節に `note: test did not panic as expected at {場所}` だけが出る。これまでの取り出し方では、`detail` が空になる。
- panic の内容が `expected` と違ったときは、panic メッセージと `note: run with` の行の後に、`note: panic did not contain expected string`、`panic message: ...`、`expected substring: ...` の行が続く。これまでの取り出し方では、`note: run with` の行で終えるので、期待した内容が `detail` に入らない。

`test` 方式の隠しテストの判定の部品（Issue #161）のレビューで見つかった。結果の行と `detail` の読み方なので、Claude が技術的な判断として決めた。

## 決定

[コードの提出（submit）](../spec/exercise-runner/submit.md)「test 方式」の 6 と 7 を、次のように変えた。

- 結果の行に、`test {名前} - {テストの種類}`（`test {名前} - should panic` など）が行末の ` ... ok` などの直前にある行も含める。
  - 理由: libtest は、テストの種類があるテストで、名前の後にその表示を付ける。表示の種類を ` - should panic` だけに絞らず、名前の後の ` - {テストの種類}` として読むと、ほかの種類の表示が付いても結果の行を見失わない。
- `fail` の panic メッセージは、最後の `panicked at` の行の次の行から次の節の見出しの前までとし、`note: run with` で始まる行は、そこで終えるのではなく除く。
  - 理由: `expected` と違ったときの libtest の説明は、`note: run with` の行の後に続く。学習者は、期待された panic の内容を `detail` で見られる。`#[should_panic]` のないテストでは、`note: run with` の行の後は空行と見出しだけなので、取り出す内容は変わらない。
- `panicked at` の行が無いときは、`note: test did not panic as expected` から行末までを `detail` にする。
  - 理由: panic しなかったことを、空の `detail` ではなく、libtest の説明で示せる。

## 検討した他の案

- 案: 結果の行の読み方だけを直し、`detail` の取り出し方は変えない
  - 採用しなかった理由: ch09-01 の演習で、panic しなかった答えの `detail` が空になり、学習者は失敗の原因を知る手がかりが無い
- 案: `#[should_panic]` の失敗では、`---- {名前} stdout ----` の節の全体を `detail` にする
  - 採用しなかった理由: 学習者の出力も入り、ほかのテストの `detail`（panic メッセージだけ）と形がそろわない

## 影響

- `#[should_panic]` の隠しテストを、ほかの隠しテストと同じように `pass`・`fail` で判定できる。
- 結果の行を読むときは、名前の後のテストの種類の表示を許す。表示の種類（`should panic` など）が libtest で変わっても、` - ` と小文字の英語の表示なら読める。
