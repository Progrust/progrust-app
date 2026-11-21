---
type: Verification Report
title: "隠しテストを読み込む #[path] の解決の技術検証"
description: "隠しテストを読み込む `#[path]` が `judge.target` の場所によらず解決されるかを Issue #14 で検証した結果（`../` の数の決め方、絶対パス案、テスト名へのモジュール経路の付加、`use super::*;` の基準、`judge.target` が `src/` 直下に限られるので仕様の値を変えなかった判断）を記す。"
---

- 日付: 2026-11-21
- Issue: [#14](https://github.com/Progrust/progrust-app/issues/14)
- 確かめた OS: macOS 27.2.1（Darwin 27.2.0、Apple Silicon）
- 関連仕様: [提出と判定](../spec/exercise-runner/submit.md)、[ADR 0009](../decisions/0009-embed-hidden-tests-as-module.md)

## 環境

- macOS 27.2.1（Darwin 27.2.0）、Apple Silicon、8 コア
- rustc 1.99.1 (58cf8f9c3 2026-10-13) stable、cargo 1.99.1

## 方法

`project/src/...` の下の指定ファイルの末尾に、仕様どおりの次の 3 行を付けた。`project/progrust_judge_tests.rs`（`use super::*;` と `#[test]` を持つ）を読み込めるかを、`cargo test --offline progrust_judge::` で確かめた。`#[path]` の値だけを変えて繰り返した。

```rust
#[cfg(test)]
#[path = "../progrust_judge_tests.rs"]
mod progrust_judge;
```

## 結果

### 仕様の固定値での結果

仕様どおりの固定値 `"../progrust_judge_tests.rs"` では、`src/` 直下のファイル以外は読み込めない。

| judge.target | モジュール階層 | `"../progrust_judge_tests.rs"` | 正しい値 | 実際のテスト名 |
| --- | --- | --- | --- | --- |
| src/main.rs | crate ルート | OK | 同左 | `progrust_judge::t_ok` |
| src/lib.rs | crate ルート | OK | 同左 | `progrust_judge::t_ok` |
| src/geo.rs | `geo` | OK | 同左 | `geo::progrust_judge::t_ok` |
| src/a.rs | `a` | OK | 同左 | `a::progrust_judge::t_ok` |
| src/a/mod.rs | `a` | NG | `../../progrust_judge_tests.rs` | `a::progrust_judge::t_ok` |
| src/a/b.rs（親が a/mod.rs） | `a::b` | NG | `../../progrust_judge_tests.rs` | `a::b::progrust_judge::t_ok` |
| src/a2/b.rs（親が src/a2.rs。新形式） | `a2::b` | NG | `../../progrust_judge_tests.rs` | `a2::b::progrust_judge::t_ok` |
| src/a/b/c.rs | `a::b::c` | NG（`../../` でも NG） | `../../../progrust_judge_tests.rs` | `a::b::c::progrust_judge::t_ok` |

NG のときのエラーは `couldn't find file src/a/../progrust_judge_tests.rs` の形になる。

### パスの決め方

`mod` 宣言に付けた `#[path]` は、常に「その宣言を書いたファイルがあるディレクトリ」からの相対で解決される。`mod.rs` かどうかや、モジュールが何階層かは関係がない（インラインモジュールの中に書かない限り）。したがって `../` の数は、「`judge.target` が `src/` から何階層のディレクトリにあるか」に 1 を足した数になる。

- `src/x.rs` は 1 個
- `src/a/x.rs`（`src/a/mod.rs` を含む）は 2 個
- `src/a/b/x.rs` は 3 個

「`mod.rs` だから基準が変わる」ことはない。`src/a.rs` は `src/` にあり、`src/a/mod.rs` は `src/a/` にあるので、ディレクトリの深さが違うだけである。

### 絶対パス案

`#[path = "/abs/path/to/project/progrust_judge_tests.rs"]` は、上の全パターン（main.rs、a/b.rs、a/mod.rs、a2/b.rs、geo.rs）で読み込めた。空白と日本語を含むディレクトリ（`日本 語 dir`）でも読み込めた。ただし次の点がある。

- コンパイルエラーなどに表示されるパスが絶対パスになる（相対パスでは `src/../progrust_judge_tests.rs`）。表示用に置き換える処理が要るかもしれない。
- プロジェクトのコピー先が変わるたびに書き換えが要る。相対パスならプロジェクトを移動しても動く。

### 仕様との食い違い

- テスト名にモジュールの経路が付く。`judge.target` が `src/main.rs`・`src/lib.rs` 以外のとき、libtest の出力は `test geo::progrust_judge::t_ok ... ok` になり、仕様の `test progrust_judge::{関数名} ... ok|FAILED` と一致しない。`progrust_judge::` で絞り込む指定は部分一致なので動作する（`geo::progrust_judge::t_ok` も選ばれる）。結果の解析は、`progrust_judge::` の手前の経路を許す必要がある。`--list` の出力で名前を先に得れば、経路を気にしなくてよい（[test 方式のテスト結果の読み取りの技術検証](issue-15-test-result-parsing.md)）。
- `use super::*;` の `super` は `judge.target` のモジュールになる。`src/a/mod.rs` を対象にしたとき、隠しテストから見えるのはその `a` の項目であり、crate ルート（main.rs）の項目ではない。教材の作成者はこれを意識する必要がある。

### 検証のあとの判断

検証のあと、ユーザーは、残った判断を Claude の推奨に任せた。Claude は次のように決めた。

- `#[path]` の値: 深さに応じた相対パスにすることを推奨した。ただし、仕様は `judge.target` を `src/main.rs` か `src/lib.rs` に限っている（[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)）。どちらも `src/` 直下なので、深さに応じた値は仕様の固定値 `../progrust_judge_tests.rs` と同じになる。検証の時点では、この限定を見落として「仕様の修正が要る」とした。仕様の `#[path]` の値は変えなかった。
- テスト名: `judge.target` が crate ルートなので、テスト名は `progrust_judge::{関数名}` になる。判定では、`--list` で得た名前の `progrust_judge::` より後を関数名とすることにした（[ADR 0101](../decisions/0101-run-each-hidden-test-in-own-process.md)）。

## 判定

基準「`judge.target` がどこにあっても隠しテストを読み込める。または、読み込めるパスの決め方が分かる」は満たした。決め方は上の規則（`../` を、`judge.target` の `src/` からのディレクトリの深さ + 1 個）である。仕様の固定値は `src/` 直下のファイル専用である。仕様は `judge.target` を `src/` 直下の 2 つに限っているので、固定値のままで読み込める（「検証のあとの判断」）。ADR 0009 の方式（末尾に子モジュールを付ける）自体は変えなくてよい。

## 残った事項

- 確かめられなかった項目: `#[cfg(test)]` を付けた `mod progrust_judge;` を追加した後の、隠しテスト側のコンパイルエラーの行番号の表示（仕様の「エラーを併せて表示」）。

ユーザーの判断が要るとした点は、検証のあとに決まった（「検証のあとの判断」）。決める前の推奨と、決まったことは次のとおり。

- `#[path]` の値: 深さに応じた相対パスを推奨した（エラー表示が `src/../progrust_judge_tests.rs` になり、プロジェクトを移動しても動く）。`judge.target` が `src/` 直下に限られるので、仕様の固定値と同じになり、仕様は変えなかった。`judge.target` に `src/` 直下以外のファイルを許すときは、`../` の数を「`judge.target` の `src/` からのディレクトリの深さ + 1」で決める必要がある。
- 仕様の変更案（テスト名に経路が付きうること）: `judge.target` が crate ルートに限られるので、経路は付かない。判定では `--list` で得た名前から関数名を取るので、経路が付いても対応できる（ADR 0101）。
