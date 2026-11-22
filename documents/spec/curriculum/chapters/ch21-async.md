---
type: Specification
title: "ch21 非同期処理"
description: "ch21 非同期処理の章の説明と、async fn・await とランタイム（tokio）、タスクの生成、複数の Future を同時に待つ方法、早く終わったものを使う方法、非同期のチャネル、非同期の共有状態、ストリーム、Future トレイトと Pin を扱うレッスン（ch21-01〜ch21-08 と章末復習）の ID・タイトル・主に扱う内容・主要素を定める。"
---

async/await と非同期ランタイム（tokio）による非同期処理を学ぶ

| ID | タイトル | 主に扱う内容 | 主要素 |
| --- | --- | --- | --- |
| ch21-01 | async fn・await とランタイム | `async fn`、Future は `await` するまで動かないこと、`#[tokio::main]` | `async.basic` |
| ch21-02 | タスクを生成する | `tokio::spawn`、`JoinHandle` | `async.spawn` |
| ch21-03 | 複数の Future を同時に待つ | `join!`・`try_join!`、数が実行時に決まる場合の `futures::future::join_all` | `async.join` |
| ch21-04 | 早く終わったものを使う | `select!`、`tokio::time::timeout`・`sleep` | `async.select_timeout` |
| ch21-05 | 非同期のチャネル | `tokio::sync::mpsc` | `async.channel` |
| ch21-06 | 非同期の共有状態 | `Arc` と `tokio::sync::Mutex`、標準の `Mutex` との使い分け | `async.shared_state` |
| ch21-07 | ストリーム | 非同期に値を順に受け取る、`futures` の `Stream` と `StreamExt`、`buffer_unordered` で同時に動かす数を制限する、`tokio-stream`（tokio の型を Stream にする）の紹介 | `async.stream` |
| ch21-08 | Future トレイトと Pin | `poll` と `Poll`、`Pin` が必要な理由、`Future` を手で実装する | `async.future_pin` |
| ch21-review | 章末復習 | | |
