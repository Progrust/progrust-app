# 教材データ形式

* [教材データ形式の目的とスコープ](scope.md) - 教材データ形式の仕様が扱う範囲（章・レッスン・動画・演習・要素マスタのファイル形式）と、教材の書き方のルールと AI が生成する演習の保存についての参照先を定める。
* [教材のディレクトリ構成](directory-structure.md) - `content/` の下の `chapters.json`・`elements.json`・`voice.json`・`readings.json`・`assets/`・`lessons/`（レッスンごとの `lesson.json`・`video/`・`exercise/`）の配置と、動画の共通部品と音声を作るツールを `content/` の外に置くことを定める。
* [章とレッスンの並びの形式（chapters.json）](chapters-json.md) - `chapters.json` の形式と、学習順・過去のレッスン・導入レッスン・既習の要素の定義、各章の最後に章末復習レッスン（`ch{NN}-review`）を置くことを定める。
* [要素マスタの形式（elements.json）](elements-json.md) - `elements.json` の各要素のフィールド（`id`・`chapter`・`name`・`description`・`related_errors`）と、要素 ID を廃止するときに `deprecated` を付けることを定める。
* [レッスン定義の形式（lesson.json）](lesson-json.md) - `lesson.json` のフィールド（`kind`・`tags`・`primary_elements`・`secondary_elements`・`estimated_minutes`・`exercise.judge`（`min_learner_tests` を含む）など）と、`kind` ごとの `tags` の要否を定める。
* [レッスンの動画の形式（video/）](video/index.md) - レッスンの動画を表す `video/` のファイル（台本、音声、シーン）の形式と、タイムラインと合図、コードの表示、色の指定、禁止事項を定める。
* [演習のファイル形式](exercise.md) - 演習のファイル（`problem.md`・`explanation.md`・`starter/`・`solution/`）、問題文と解説に持たせる見出し、`stdout` 方式の `cases.json` と `test` 方式の `judge_tests.rs` の書き方を定める。
* [教材データ形式の受け入れ条件](acceptance.md) - 教材データ形式の仕様を満たしたと判断する条件（教材チェックツールで検出するエラー、動画の色・行番号・音声・合図の振る舞い）を定める。
* [教材データ形式の未決事項](open-questions.md) - 教材データ形式の仕様でまだ決まっていない事項（アプリの画面の CSS が動画の中にも効くことの扱い、レッスン専用の CSS で動画のデザインシステムの部品を上書きしてよいか）を記す。
