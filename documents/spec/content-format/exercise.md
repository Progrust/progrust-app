---
type: Specification
title: "演習のファイル形式"
description: "演習のファイル（`problem.md`・`explanation.md`・`starter/`・`solution/`）、問題文と解説に持たせる見出し、`stdout` 方式の `cases.json` と `test` 方式の `judge_tests.rs` の書き方、C の演習のファイル（`main.c` と `cases.json` だけ）を定める。"
---

## ファイル

この節の表は、Rust の演習のファイルである。C の演習は「C の演習」に定める。

| ファイル | 内容 |
|---|---|
| `problem.md` | 問題文（Markdown） |
| `explanation.md` | 解説（Markdown） |
| `starter/` | 初期コード。cargo パッケージのルートからの相対パスで配置する（例: `starter/src/main.rs`）。`Cargo.toml` は置かない（アプリが生成する）。`src/main.rs` を必ず含める |
| `solution/` | 模範解答。構成は `starter/` と同じ |
| `cases.json` | `stdout` 方式のテストケース（「判定方式」） |
| `judge_tests.rs` | `test` 方式の隠しテスト（「判定方式」） |

## 問題文（problem.md）

以下の見出しを持つ。

| 見出し | 内容 |
|---|---|
| `## 問題` | 何を作るか |
| `## 入力` | 標準入力の形式（`stdout` 方式で入力がある場合） |
| `## 出力` | 出力の形式（`stdout` 方式）、または実装する関数のシグネチャと仕様（`test` 方式） |
| `## 例` | 入出力例、または呼び出し例 |
| `## 制約` | 使ってはいけないもの、守るべき条件など（任意） |

## 解説（explanation.md）

以下の見出しを持つ。

| 見出し | 内容 |
|---|---|
| `## 考え方` | 解き方の方針 |
| `## 模範解答` | 模範解答のコード（`solution/` と同じ内容） |
| `## ポイント` | このレッスンの主要素がどう使われているか |
| `## よくある間違い` | 想定される誤りと、そのときに出るエラー |

## 判定方式

この節は、Rust の演習の判定方式である。C の演習は「C の演習」に定める。

**`stdout` 方式**: プログラムに標準入力を与えて実行し、標準出力を期待値と比較する。

```json
[
  { "name": "基本の例", "stdin": "3\n1 2 3\n", "expected_stdout": "6\n" },
  { "name": "空の入力", "stdin": "0\n\n", "expected_stdout": "0\n" }
]
```

| フィールド | 説明 |
|---|---|
| `name` | テストケースの表示名 |
| `stdin` | 標準入力として与える文字列 |
| `expected_stdout` | 期待する標準出力 |

**`test` 方式**: 隠しテストを学習者のコードに組み込んで `cargo test` で実行する。

```rust
// judge_tests.rs
use super::*;

#[test]
fn empty_input() {
    assert_eq!(sum(&[]), 0);
}
```

- `judge.target` のファイルの子モジュールとして組み込まれるため、`use super::*;` で学習者のコードを参照する。
- テスト関数名は snake_case の英語とし、表示名は `lesson.json` の `case_labels` で与える。
- 比較方法・組み込み方法・制限時間は [コードの提出（submit）](../exercise-runner/submit.md) に定める。
- `lesson.json` の `exercise.judge.min_learner_tests` を書いた演習では、隠しテストに加えて、学習者が書いたテスト（単体テスト、`tests/` の統合テスト、ドキュメントテスト）がすべて通り、その件数が指定以上であることも判定する（[コードの提出（submit）](../exercise-runner/submit.md)「学習者のテストの判定」）。
- 統合テストとドキュメントテストを書かせる演習では、`starter/` に `src/lib.rs` を含め、`judge.target` を `src/lib.rs` にする。統合テストのファイルは `tests/` に置き、`progrust_exercise` クレートとして `src/lib.rs` の公開アイテムを使う。

## C の演習

C のレッスンの演習（[レッスン定義の形式（lesson.json）](lesson-json.md)「C のレッスン」）のファイルは、次のとおりである。

| ファイル | 内容 |
|---|---|
| `problem.md` | 問題文。見出しは Rust の演習と同じ |
| `explanation.md` | 解説。見出しは Rust の演習と同じ |
| `starter/main.c` | 初期コード。パッケージを作らないので、`src/` も `Cargo.toml` も無い |
| `solution/main.c` | 模範解答 |
| `cases.json` | テストケース。書き方は `stdout` 方式と同じ |

- ファイルは `main.c` の 1 つだけである。`judge_tests.rs` は置かない。
- 判定は `stdout` 方式だけで、テストケースごとに標準入力を与えて標準出力を期待値と比較する。提出の手順（ビルド、メモリの検査、`memory_error` の判定を含む）は [C のコードの提出（submit）](../exercise-runner/c-submit.md) に従う。
