# コマンドとイベント

* [コマンドとイベントの共通の決まり](conventions.md) - フロントエンドとバックエンドの間のコマンド（`invoke`）とイベント（`emit`）に共通する決まり（名前の付け方、引数と戻り値の JSON の書き方と型の表し方、日時の書き方、エラーの形（`code`・`message`・`detail`）とエラーの種類、AI 呼び出しのエラーの形、要求を見分ける `request_id`、イベントの宛先）を定める。
* [環境のコマンドとイベント](environment.md) - `cargo`・`claude` の検出結果と初回準備の状態を表す環境の状態の形と、`get_environment_status`・`redetect_tools`・`retry_crate_preparation` の引数と戻り値、`environment_updated` の内容を定める。
* [教材のコマンド](content.md) - 教材のコマンド `list_chapters`（章とレッスンの一覧と、レッスンごとの `in_preparation`）、`get_lesson`（レッスン定義・台本・音声のディレクトリ・演習の内容。生成問題は `gen-{generation_id}` で開く）、`search_lessons`（検索の結果と強調する位置）の引数と戻り値を定める。
* [進捗のコマンド](progress.md) - 進捗のコマンド `get_progress`（進捗と推奨レッスン）、`open_lesson`（試行セッションの開始と進捗・下書きの読み込み）、`end_session`（画面の操作による試行セッションの終了）、`save_draft`・`reset_draft`、`get_passed_code`、`save_video_position` の引数と戻り値を定める。
* [実行のコマンドとイベント](run.md) - 実行のコマンド `run_code`（`mode` で実行とテスト実行を選ぶ）、`send_stdin`・`close_stdin`・`stop_run`、`submit_code` の引数と戻り値と、イベント `run_started`（実行で学習者のプログラムを起動したこと）・`run_output`（出力の断片の `stream`）・`submit_progress`（判定の進み具合）・`run_finished`（実行の結果の状態と提出の判定結果）の内容を定める。
* [質疑応答のコマンドとイベント](chat.md) - 質疑応答のコマンド `send_chat`（質問の場面 `context`、再生中の文の ID、フロントエンドが持つ会話履歴 `history`、再試行かどうか `retry`）と `cancel_chat` の引数と戻り値、イベント `chat_delta`（回答の断片）と `chat_finished`（`completed`・`cancelled`・`error`）の内容を定める。
* [問題生成のコマンド](generation.md) - 問題生成のコマンド `list_generated`（生成問題一覧）、`get_generation_options`（生成設定画面の要素の一覧と習熟レベル、初期値の材料）、`request_generation`（生成の依頼と `generation_id`）、`delete_generated` の引数と戻り値を定める。
* [学習記録・分析のコマンド](learning-record.md) - 学習記録・分析のコマンド `get_dashboard`（トップページの次にやるべきレッスン・完了数・章ごとの完了率・分析の要約と分析の状態 `none`・`ok`・`unreadable`）、`list_unreadable_records`、`rebuild_analysis`、`reset_analysis`、`reset_all` の引数と戻り値を定める。
* [ジョブのコマンドとイベント](jobs.md) - ジョブの形（`jobs.json` のジョブの項目と、対象の表示名 `target_title`）と、ジョブのコマンド `list_jobs`・`retry_job` の引数と戻り値、イベント `job_updated` の内容を定める。
* [設定のコマンド](settings.md) - 設定のコマンド `get_settings`・`update_settings`（`settings.json` の全体）、`open_in_os`（開く対象 `records`・`analysis`・`logs`）、`test_claude_connection`（接続テストの結果）、`get_app_version` の引数と戻り値を定める。
