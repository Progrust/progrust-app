---
type: Decision
title: "ADR 0147"
description: "フロントエンドとバックエンドの間のコマンドとイベントの入力・出力の形を `spec/architecture/commands/` の 1 か所に定め、要求をフロントエンドが作る `request_id` で見分け、エラーを共通の形で返す"
---

- 日付: 2026-11-22
- 関連仕様: [コマンドとイベント](../spec/architecture/commands/index.md)、[フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)「コマンドとイベント」、[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)、[環境のコマンドとイベント](../spec/architecture/commands/environment.md)、[教材のコマンド](../spec/architecture/commands/content.md)、[進捗のコマンド](../spec/architecture/commands/progress.md)、[実行のコマンドとイベント](../spec/architecture/commands/run.md)、[質疑応答のコマンドとイベント](../spec/architecture/commands/chat.md)、[問題生成のコマンド](../spec/architecture/commands/generation.md)、[学習記録・分析のコマンド](../spec/architecture/commands/learning-record.md)、[ジョブのコマンドとイベント](../spec/architecture/commands/jobs.md)、[設定のコマンド](../spec/architecture/commands/settings.md)、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」
- 置き換え: なし

## 背景

[フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)「コマンドとイベント」には、コマンド 31 個とイベント 6 個の名前と担当仕様の表があり、「入力・出力の詳細と動作は「担当仕様」の仕様書に従う」と書かれていた。しかし、担当仕様には、ほとんどのコマンド・イベントの入力・出力の形が書かれていなかった。名前で `documents/spec/` を検索すると、大半のコマンドは表にしか出てこなかった。このままでは、フロントエンドとバックエンドを実装するときに、両者の形がずれるおそれがあった。

ほかにも、次のことが決まっていなかった。

- 表に無い操作があった。演習画面の「テスト」ボタン（[コードの実行（run）](../spec/exercise-runner/run.md)「テスト実行」）と、初回準備の「再試行」（[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「準備中・失敗時の動作」、[設定画面と settings.json](../spec/screens/settings-screen.md)）である。
- 実行・提出・質疑応答で、`run_output`・`run_finished`・`chat_delta` がどの要求のものかを見分ける方法。
- 提出の判定結果と「判定の進み具合」（[コードの提出（submit）](../spec/exercise-runner/submit.md)）の送り方。
- `get_environment_status`・`environment_updated` の「初回準備の状態」の値。`prepared.json` の `status` は `ready` と `failed` だけで、準備中を表す値が無かった。
- `send_chat` の入力と、会話履歴をフロントエンドとバックエンドのどちらが持つか。
- 生成問題を何で開くか、`get_dashboard` の形、`open_in_os` が何を受けるか、`test_claude_connection` の結果の形、エラーの返し方の共通の形。

プロジェクトの方針（`CLAUDE.md`）で、アプリの操作感に関わらない技術の詳細は、Claude が最適と思う方法で決めることになっている。以下の決定と理由は、すべて Claude の判断である。

## 決定

- コマンドとイベントの入力・出力の形を、新しいディレクトリ `spec/architecture/commands/` に定める。共通の決まりを 1 つの文書に、分類ごとの形をそれぞれの文書に置く（[コマンドとイベント](../spec/architecture/commands/index.md)）。名前と担当仕様の表は [フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)「コマンドとイベント」に残し、動作は引き続き担当仕様に従う。既に仕様に形があるデータ（`lesson.json`、`progress.json`、`meta.json`、提出の判定結果など）は、書き写さずに参照する。
  - 理由: 1 つのコマンドの形が、複数の機能の仕様にまたがることがある（例: `run_finished` は判定結果と進捗の更新を運ぶ）。担当仕様に分けて書くと、共通の決まり（エラー、ID）を各所に書くことになり、今回のように書かれないままになりやすい。実装する人が 1 か所を見れば形が分かるようにする。
- 項目名を snake_case にし、すべてのコマンドに `rename_all = "snake_case"` を付ける（[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)「引数と戻り値」）。
  - 理由: 既存の仕様の JSON（`progress.json`、`lesson.json` など）が snake_case で、それをそのまま返す。Tauri 2 の引数名の既定（camelCase）のままだと、引数だけ書き方が違ってしまう。
- エラーを `{ code, message, detail }` の共通の形で返し、`code` の種類を共通の決まりに列挙する。AI 呼び出しのエラーは `{ category, detail }` の形で表す。コマンドが受け付けた後の失敗は、`Err` ではなく終わりのイベントで知らせる（[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)「エラー」）。
  - 理由: フロントエンドが、エラーの種類で表示を分けられる。AI のエラーの表示は [AI 呼び出しのエラーの分類と表示](../spec/claude-integration/errors.md) が分類ごとに決めているので、分類をそのまま渡す。
- 実行・テスト実行・提出・質疑応答の要求を、フロントエンドが作る UUID の `request_id` で見分ける。処理を始めるコマンドの引数で渡し、バックエンドはその要求のイベントのすべてに入れる（[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)「要求を見分ける ID」）。
  - 理由: バックエンドが ID を作ってコマンドの戻り値で返すと、戻り値より先に届いたイベントを、どの要求のものか見分けられない。
- イベントはすべてアプリの 1 つのウィンドウに送り、`environment_updated` と `job_updated` は変わった後の状態を丸ごと入れる。
  - 理由: ウィンドウは 1 つだけである。状態を丸ごと送れば、フロントエンドは差分をたどらずに表示を置き換えられる。
- テスト実行を、新しいコマンドにせず、`run_code` の引数 `mode`（`main` / `test`。試行ログの `run` イベントの `mode` と同じ値）で選ぶ（[実行のコマンドとイベント](../spec/architecture/commands/run.md)）。
  - 理由: テスト実行は、出力の送り方・停止・制限値・結果の状態・試行ログを実行と共有している（[ADR 0112](0112-run-learner-tests-with-test-button.md)）。同じコマンドにすれば、`send_stdin`・`stop_run`・`run_finished` も同じものを使える。
- 提出の判定の進み具合を、新しいイベント `submit_progress`（段階、終わったテストケースの数、全体の数）で送る。判定結果は `run_finished` の `submit` に入れ、ビルドの出力は実行と同じく `run_output` で送る。
  - 理由: 進み具合を `run_output` の文として送ると、フロントエンドが件数を取り出せず、表示の作り方を選べない。
- 初回準備の再試行のコマンド `retry_crate_preparation` を加える（[環境のコマンドとイベント](../spec/architecture/commands/environment.md)）。
- 初回準備の状態を、バックエンドがメモリに持つ `not_started`・`preparing`・`ready`・`failed` の 4 つの値で表し、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」に定める。`prepared.json` の `status` は変えない。
  - 理由: 準備中は、アプリが動いている間だけの状態である。`prepared.json` に準備中を書くと、準備の途中でアプリが落ちたときに準備中のまま残り、起動時の「実行する条件」（`failed` のときにやり直す）に当てはまらなくなる。`cargo` が見つからない間も状態を表せるよう、`not_started` を置く。
- 質疑応答の会話履歴は、フロントエンドが画面の状態として持ち、質問のたびに `send_chat` の `history` で渡す。バックエンドは会話履歴を持たない（[質疑応答のコマンドとイベント](../spec/architecture/commands/chat.md)）。
  - 理由: 会話履歴は画面を開いている間だけ保持する（[ADR 0025](0025-chat-history-only-while-open.md)）。画面の出入りを知っているのはフロントエンドである。バックエンドが持つと、画面を移ったときに捨てさせるコマンドが別に要る。
- 生成問題は、レッスン ID `gen-{generation_id}` で、レッスンと同じ `get_lesson`・`open_lesson`・`run_code`・`submit_code` などで扱う。
  - 理由: 生成問題はレッスンの演習と同じ形式で保存し、試行ログでも `gen-{generation_id}` をレッスン ID として使っている（[生成問題の保存](../spec/exercise-generation/storage.md)）。
- 画面が要るデータか合図を運ぶものが無かった次のコマンドを加える。
  - `end_session`: 試行セッションの終了の条件のうち、画面の操作によるもの（[試行ログ（attempts/*.json）](../spec/learning-record-and-analysis/attempt-log.md)「試行セッションのライフサイクル」）は、バックエンドからは見えない（[進捗のコマンド](../spec/architecture/commands/progress.md)）。
  - `get_generation_options`: 生成設定画面に、要素マスタと習熟レベルと初期値の材料が要る（[問題生成のコマンド](../spec/architecture/commands/generation.md)）。
  - `list_unreadable_records`: 設定画面の「読み込めない学習記録」の一覧が要る（[学習記録・分析のコマンド](../spec/architecture/commands/learning-record.md)）。
  - `get_app_version`: 設定画面にバージョンを出すが、フロントエンドにはアプリのコマンド以外の Tauri の権限を許可しない（[Tauri の権限](../spec/architecture/tauri-permissions.md)、[設定のコマンド](../spec/architecture/commands/settings.md)）。
- `open_in_os` は、パスではなく、開く対象の名前（`records`・`analysis`・`logs`）を受ける。
  - 理由: 任意のパスを受けると、フロントエンドから学習者の PC のどのファイルでも開けてしまう。フロントエンドに許可する権限を最小限にする方針（[Tauri の権限](../spec/architecture/tauri-permissions.md)）に合わせる。
- `get_dashboard` は、トップページに出す要素や章を、バックエンドが選んで並べた形で返す。
  - 理由: 集計はバックエンドで行う（[フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)）。
- `test_claude_connection` は、AI の呼び出しが失敗しても `Err` にせず、`ok` と AI 呼び出しのエラーを戻り値に入れる。
  - 理由: 接続テストでは、失敗も確かめた結果の 1 つで、分類ごとの表示に使う。

## 検討した他の案

- 案: 入力・出力の形を、それぞれの担当仕様の仕様書に書く（それまでの表の文のとおり）
  - 採用しなかった理由: 「決定」の 1 つ目の理由のとおり。共通の決まりと、機能をまたぐ形の置き場所が決まらない
- 案: ストリーミングを、イベントではなく Tauri 2 のチャンネル（`Channel`）で、要求ごとに送る
  - 採用しなかった理由: ID が要らなくなるが、`job_updated`・`environment_updated` のような、要求に結びつかない知らせにはイベントが要り、仕組みが 2 つになる。表のイベントの名前は既に仕様にある
- 案: 要求の ID をバックエンドが作り、処理を始めるコマンドの戻り値で返す
  - 採用しなかった理由: 戻り値より先に届いたイベントを見分けられない
- 案: テスト実行を別のコマンド（`run_tests` など）にする
  - 採用しなかった理由: 標準入力・停止・結果のコマンドとイベントを、実行と別に持つか共有するかを決める必要が増える
- 案: `prepared.json` の `status` に準備中の値を加える
  - 採用しなかった理由: 準備の途中でアプリが落ちると、準備中のまま残る
- 案: バックエンドが会話を ID ごとに持ち、`send_chat` には今回の質問だけを渡す
  - 採用しなかった理由: 画面を移ったときに会話を捨てさせるコマンドが要る。アプリを長く使うと、捨て忘れた会話がメモリに残る
- 案: 判定の進み具合を、`run_output` の `system` の文で送る
  - 採用しなかった理由: フロントエンドが件数を扱えない
- 案: `open_in_os` に開くパスを渡す
  - 採用しなかった理由: 任意のファイルを開けてしまう

## 影響

- フロントエンドとバックエンドは、`spec/architecture/commands/` の形に合わせて実装する。形を変えるときは、そのディレクトリの文書を直す。
- コマンドが 31 個から 36 個に、イベントが 6 個から 7 個に増えた。
- 次のことは、この判断では決めていない。いまの仕様の文のとおりにし、決まったら担当仕様と、必要なら形を直す。
  - 提出を途中で止められるか。`stop_run` は、実行・テスト実行を止める。
  - 実行・提出の途中で画面を離れたとき、アプリを終了したときの扱い。
  - 質疑応答の「再試行」を、試行ログの質問として数えるか。会話履歴に、止めた回答やエラーになった往復を入れるか。
  - 生成中の問題の削除と、「学習履歴をすべてリセット」のときの生成中の問題の扱い。
  - 失敗した generate ジョブの再実行と、生成問題一覧の「再生成」の関係。
  - `cargo` が再検出で見つかったときに、初回準備を始めるか。
- ジョブの `last_error` の中の形は [ジョブキュー（jobs.json）](../spec/claude-integration/job-queue.md) に定まっていないので、`list_jobs` などはそのまま返す。
