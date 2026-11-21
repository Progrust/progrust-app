# カリキュラム

* [カリキュラムの目的とスコープ](scope.md) - カリキュラムの仕様が扱う範囲と、章とレッスンの ID・タイトル・章の説明・学習順をこの仕様を唯一の定義とすること、主要素・副要素は `lesson.json` で定めること、教材の作成ルールと `chapters.json` の形式の参照先を定める。
* [カリキュラムの方針](policy.md) - Rust 固有でない内容はまとめて扱い Rust 固有の内容は細かく分けること、The Rust Programming Language（TRPL）を網羅の目安とし文章・コード・題材は使わないこと、主要素の数と章末復習レッスンの置き方の参照先を定める。
* [章とレッスンの一覧](chapters/index.md) - ch01〜ch23 の各章について、章の説明と、レッスンの ID・タイトル・主に扱う内容を学習順に定める。
* [カリキュラムで扱わない内容](out-of-scope.md) - どの章でも扱わない内容（Rust のインストールと rustup、crates.io への公開・ワークスペース・`cargo install`・リリースプロファイル、手続き的マクロ、FFI、ファイルの読み書き・環境変数・コマンドライン引数）を定める。
* [レッスンの演習を作るときの注意](exercise-constraints.md) - 判定方式から来る制約で注意が必要な箇所（ch01〜ch02、ch03〜ch08 の標準入力、ch09-01 の panic、main から Err を返す演習、HashMap・HashSet の走査の順序、ch10-06 の rand、パッケージ名 `progrust_exercise`、ch20・ch21 の出力の順序、ch11 の学習者のテスト）ごとに、制約と対応を定める。
* [カリキュラムの受け入れ条件](acceptance.md) - `content/chapters.json` と各レッスンの `lesson.json` の `title` が章とレッスンの一覧と一致することと、扱わない内容を主要素とするレッスンがないことを、カリキュラムの仕様を満たしたと判断する条件として定める。
* [カリキュラムの未決事項](open-questions.md) - カリキュラムの仕様でまだ決まっていない事項（許可クレートの追加（`thiserror`・`anyhow`・`tokio`・`futures`・`tokio-stream`）、ch01〜ch02 のテストケースの件数、標準入力のおまじないの定型）を記す。
