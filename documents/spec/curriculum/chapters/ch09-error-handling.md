---
type: Specification
title: "ch09 エラー処理"
description: "ch09 エラー処理の章の説明と、panic と表明、Result 型と unwrap・expect、? 演算子、文字列を数値に変換する方法、標準入力の読み方、main から Result を返す方法を扱うレッスン（ch09-01〜ch09-06 と章末復習）の ID・タイトル・主に扱う内容・主要素を定める。"
---

回復できないエラーと回復できるエラーの扱い方を学ぶ

| ID | タイトル | 主に扱う内容 | 主要素 |
| --- | --- | --- | --- |
| ch09-01 | panic と表明 | `panic!`、panic が起きる場面、`assert!`・`assert_eq!`・`unreachable!`・`todo!` | `error.panic` |
| ch09-02 | Result 型と unwrap・expect | `Ok`・`Err`、`match` によるエラーの処理、失敗したら panic する取り出し方と使ってよい場面 | `error.result` |
| ch09-03 | ? 演算子 | エラーを呼び出し元に返す、`Option` での `?`、`ok_or` で `Option` を `Result` にする | `error.question_mark` |
| ch09-04 | 文字列を数値に変換する | `parse`、型の指定（ターボフィッシュ `::<>`）、変換の失敗 | `string.parse` |
| ch09-05 | 標準入力を読む | `std::io::stdin`、`read_line`・`lines`、入力の終わり、`print!` で出したプロンプトを入力の前に表示する `flush` | `io.stdin` |
| ch09-06 | main から Result を返す | `fn main() -> Result<(), E>`、終了コード | `error.main_result` |
| ch09-review | 章末復習 | | |
