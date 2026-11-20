---
type: Specification
title: "ch19 エラー処理の発展"
description: "ch19 エラー処理の発展の章の説明と、Option と Result のメソッド、独自のエラー型、? によるエラーの変換、種類の異なるエラーをまとめて返す方法、thiserror と anyhow を扱うレッスン（ch19-01〜ch19-06 と章末復習）の ID・タイトル・主に扱う内容を定める。"
---

エラーを変換・合成するメソッドと、独自のエラー型、エラー処理のクレートを学ぶ

| ID | タイトル | 主に扱う内容 |
| --- | --- | --- |
| ch19-01 | Option と Result のメソッド | `map`・`and_then`・`unwrap_or`・`unwrap_or_else`・`map_err`・`ok`・`ok_or_else`、`Result` の `collect` |
| ch19-02 | 独自のエラー型 | 列挙型のエラー、`Display` と `std::error::Error` の実装 |
| ch19-03 | ? によるエラーの変換 | `From` を実装して `?` で自動変換する |
| ch19-04 | Box&lt;dyn Error&gt; | 種類の異なるエラーをまとめて返す、独自のエラー型との使い分け |
| ch19-05 | thiserror でエラー型を定義する | `#[derive(Error)]`、`#[error(...)]`、`#[from]` |
| ch19-06 | anyhow でエラーを扱う | `anyhow::Result`、`context` で文脈を付ける、`bail!`、ライブラリとアプリケーションでの使い分け |
| ch19-review | 章末復習 | |
