# 学習記録・分析

* [学習記録・分析の目的とスコープ](scope.md) - 学習記録・分析の仕様が扱う範囲と、`claude -p` の呼び出し方とジョブキュー、データの使い道の参照先を定める。
* [学習記録・分析の設計方針](design-principles.md) - 事実（アプリが計算）と解釈（AI が判断）の分離、Markdown + YAML frontmatter のファイル形式、共通の要素 ID、AI にファイルを直接書かせない方針を定める。
* [学習記録・分析の全体フロー](flow.md) - 試行ログへの追記から記録ジョブと分析ジョブを経て、学習記録と `analysis.md` を保存するまでの流れを定める。
* [試行ログ（attempts/*.json）](attempt-log.md) - 試行ログ `attempts/*.json` の試行セッションの開始・継続・終了の条件と、`schema_version`・`chat`・`run`・`submit` イベントを含む JSON のスキーマ、`run` の `exit_code` の意味、読み込めない試行ログの扱いを定める。
* [学習記録ファイル（records/*.md）](record-file.md) - 学習記録ファイル `records/*.md` のファイル名（`record_id`）、frontmatter のアプリ管理フィールドと AI 管理フィールド（`outcome`、`stumbles` など）、本文のセクションを定める。
* [分析ファイル（analysis/analysis.md）](analysis-file.md) - 分析ファイル `analysis/analysis.md` の習熟レベル、アプリによる `base_level` の計算、AI による `level` の調整の制約、`trend`・`review_due` などの計算方法、frontmatter と本文を定める。
* [学習記録・分析の AI ジョブ](ai-jobs.md) - 記録セッションと分析セッションの入力、試行ログの圧縮、分析セッションに出力させるフィールド、出力の検証と保存、分析の再構築（rebuild）の手順を定める。
* [学習記録・分析ファイルの手動編集への対応](manual-edit.md) - ユーザーが手で編集した学習記録・分析ファイルを解析できないときの、集計からの除外と警告、再構築の案内を定める。
* [学習記録・分析ファイルのスキーマのバージョン管理](schema-version.md) - 学習記録・分析ファイルが持つ `schema_version` と、スキーマを変えたときに古いファイルを検出して記録の再生成や分析の再構築を案内することを定める。
* [学習記録・分析の受け入れ条件](acceptance.md) - 学習記録・分析の仕様を満たしたと判断する条件を定める。
* [学習記録・分析の未決事項](open-questions.md) - 学習記録・分析の仕様でまだ決まっていない事項（`active_minutes` で動画の視聴に使った時間をどう扱うか、`review_due` と `last_4_weeks` を表示のときに計算し直すか、分析の再構築で複数の記録を渡す形と途中で失敗したときの扱い、形式が古いファイルについての案内を出す場所）を記す。
