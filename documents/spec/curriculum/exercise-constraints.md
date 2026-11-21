---
type: Specification
title: "レッスンの演習を作るときの注意"
description: "判定方式から来る制約で注意が必要な箇所（ch01〜ch02、ch03〜ch08 の標準入力、ch09-01 の panic、main から Err を返す演習、HashMap・HashSet の走査の順序、ch10-06 の rand、パッケージ名 `progrust_exercise`、ch20・ch21 の出力の順序、ch11 の学習者のテスト）ごとに、制約と対応を定める。"
---

レッスンごとに演習を作るときに、判定方式（[コード実行・判定](../exercise-runner/index.md)）から来る制約で注意が必要な箇所。

| 箇所 | 制約 | 対応 |
| --- | --- | --- |
| ch01〜ch02 | 関数が未習なので `stdout` 方式しか使えない。標準入力の読み込みも未習 | 入力のない、出力が固定の演習にする。「コンパイルエラーになるコードを直す」「途中まで書かれたコードを完成させる」のように、主要素を使わないと解けない問題にする。テストケースの件数は [カリキュラムの未決事項](open-questions.md) |
| ch03〜ch08 | 標準入力の読み込み（ch09-05）が未習 | `test` 方式（隠しテストが関数を呼ぶ）を中心にする。`stdout` 方式で入力が要る場合だけ、おまじない（[未登場の要素を使わない](../content-guidelines/no-unseen-elements.md)）を使う |
| ch09-01 panic | `stdout` 方式では panic が `runtime_error`（[コードの実行（run）](../exercise-runner/run.md)「結果の状態」）になり、「正しく panic する」ことを判定できない | `test` 方式の隠しテストで `#[should_panic]` を使う |
| ch09-06・ch19 の main から Err を返す演習 | 終了コード 0 以外は `runtime_error` になる | `fn run() -> Result<..>` を実装させ、`test` 方式で判定する |
| ch08-08〜ch08-10 HashMap・HashSet | 走査の順序が実行ごとに変わる | キーを指定して引く問題にする、またはキーを `Vec` に集めて `sort` してから出力させる（`collect` は ch15 なので `for` で `push`） |
| ch10-06 rand | 乱数の出力は `stdout` 方式で判定できない | `test` 方式で、値が範囲に入っていることや個数を検査する。`rand::random_range` を使えば、トレイト（ch12）を使わずに済む |
| ch10-05・ch10-06 | パッケージ名は `progrust_exercise` に固定で、`Cargo.toml` はアプリが生成し、学習者は編集できない（[実行・提出時のプロジェクトの組み立て](../exercise-runner/project-assembly.md)） | ライブラリは `use progrust_exercise::...` で使う。`[dependencies]` は動画で見せるだけにする |
| ch20・ch21 | 出力の順序が非決定的になりやすい | `join` の後にまとめて出力する、チャネルで集めてから並べ替える、待ち時間の差を十分に取る（1 ケースの制限時間内で） |
| ch11 | 隠しテストだけでは、学習者がテストを書いたかどうかを判定できない | `test` 方式で `min_learner_tests`（[レッスン定義の形式（lesson.json）](../content-format/lesson-json.md)）を書き、学習者が書いたテストがすべて通り、指定の件数以上あることも判定する。実装する関数は隠しテストで判定する。ch11-05・ch11-06 では `starter/` に `src/lib.rs` を含め、統合テストは `tests/` に、ドキュメントテストは `src/lib.rs` の公開アイテムのドキュメントコメントに書かせる（[演習のファイル形式](../content-format/exercise.md)「判定方式」） |
