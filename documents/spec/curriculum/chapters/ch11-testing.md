---
type: Specification
title: "ch11 テスト"
description: "ch11 テストの章の説明と、テスト関数、テストモジュール、panic を期待するテスト、Result を返すテスト、統合テスト、ドキュメントコメントとドキュメントテストを扱うレッスン（ch11-01〜ch11-06 と章末復習）の ID・タイトル・主に扱う内容・主要素を定める。"
---

テストを書いて、コードが期待どおりに動くことを確かめる方法を学ぶ

| ID | タイトル | 主に扱う内容 | 主要素 |
| --- | --- | --- | --- |
| ch11-01 | テスト関数を書く | `#[test]`、`assert_eq!` などでの検査、`cargo test` の結果の読み方 | `test.basic` |
| ch11-02 | テストモジュール | `#[cfg(test)]`、`mod tests` と `use super::*`、非公開の関数のテスト | `test.module` |
| ch11-03 | panic を期待するテスト | `#[should_panic]`、`expected` によるメッセージの確認 | `test.should_panic` |
| ch11-04 | Result を返すテスト | テスト関数で `?` を使う | `test.result` |
| ch11-05 | 統合テスト | `tests/` ディレクトリ、ライブラリクレートを外から使うテスト | `test.integration` |
| ch11-06 | ドキュメントコメントとドキュメントテスト | `///`・`//!`、ドキュメントの中のコード例がテストとして動く | `test.doc` |
| ch11-review | 章末復習 | | |
