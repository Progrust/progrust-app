---
type: Specification
title: "実行のコマンドとイベント"
description: "実行のコマンド `run_code`（`mode` で実行とテスト実行を選ぶ）、`send_stdin`・`close_stdin`・`stop_run`、`submit_code` の引数と戻り値と、イベント `run_started`（実行で学習者のプログラムを起動したこと）・`run_output`（出力の断片の `stream`）・`submit_progress`（判定の進み具合）・`run_finished`（実行の結果の状態と提出の判定結果）の内容を定める。"
---

動作は [コード実行・判定](../../exercise-runner/index.md) に従う。Rust の演習と C の演習で手順が違う（[コード実行・判定の概要](../../exercise-runner/overview.md)）。共通の決まりは [コマンドとイベントの共通の決まり](conventions.md) に従う。

- 実行・テスト実行・提出は、要求ごとに `request_id`（[コマンドとイベントの共通の決まり](conventions.md)「要求を見分ける ID」）で見分ける。
- 学習者による実行・テスト実行・提出は、C の演習と Rust の演習を合わせて、同時に 1 つだけ行う（[コード実行・判定の概要](../../exercise-runner/overview.md)）。動いているものがあるときに `run_code`・`submit_code` を呼ぶと、`busy` を返す。

## run_code

演習画面の「実行」と「テスト」で呼ぶ。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | この実行の ID |
| `lesson_id` | `string` | レッスン ID（生成問題は `gen-{generation_id}`） |
| `mode` | `"main" \| "test"` | `main` は実行（[コードの実行（run）](../../exercise-runner/run.md)「手順」）、`test` はテスト実行（同じく「テスト実行」）。試行ログの `run` イベントの `mode` と同じ値。C の演習では `main` だけ（[C のコードの実行（run）](../../exercise-runner/c-run.md)「テスト実行が無いこと」） |
| `files` | `{ [path: string]: string }` | エディタ上の全ファイル |

- 戻り値: `null`。実行を始めたら返す。`mode` が `main` の実行では、学習者のプログラムを起動したことを `run_started` で知らせる。出力は `run_output`、結果は `run_finished` で知らせる。
- 始める前に、`files` を下書きとして保存する（[演習コードの下書き](../../lesson-flow/drafts.md) の、実行・提出の直前の保存）。
- Rust の演習のエラー: ファイルパスが [実行・提出時のプロジェクトの組み立て](../../exercise-runner/project-assembly.md) の条件を満たさないときは `invalid_argument`。`cargo` が見つからないときは `cargo_unavailable`。許可クレートを使う演習で初回準備の状態が `ready` でないときは `crates_not_ready`。
- C の演習のエラー: ファイルパスが [C の演習の作業領域とビルド](../../exercise-runner/c-project-assembly.md)「作業領域」の条件を満たさないときと、`mode` が `test` のときは `invalid_argument`。`clang` が見つからないときは `clang_unavailable`。`invalid_argument` に当たるときは、`clang_unavailable` より先に `invalid_argument` を返す。`cargo_unavailable` と `crates_not_ready` は返さない。

## send_stdin

実行中のプログラムの標準入力に書き込む（[コードの実行（run）](../../exercise-runner/run.md)「対話的な標準入力」）。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | `run_code` で渡した ID |
| `text` | `string` | 書き込む文字列。入力欄の 1 行に改行（`\n`）を付けたもの |

- 戻り値: `null`。
- エラー: その ID の実行が動いていないとき（無いとき、既に終わったとき）と、プログラムがまだ起動していないとき（ビルド中など、`run_started` を送る前）、標準入力を閉じた後のときは `invalid_state`。テスト実行と提出の ID も `invalid_state` にする。
- `run_started` を受け取った後は、プログラムが終わるか標準入力を閉じるまで、書き込める。

## close_stdin

「入力終了」で、実行中のプログラムの標準入力を閉じる。

- 引数: `request_id`（`string`）。
- 戻り値: `null`。
- エラー: `send_stdin` と同じ。

## stop_run

停止ボタンで、実行・テスト実行を止める（[コードの実行（run）](../../exercise-runner/run.md)「停止」）。実行・テスト実行・提出の途中で演習画面を離れるときにも呼ぶ（[コードの提出（submit）](../../exercise-runner/submit.md)「途中で止めた提出」）。

- 引数: `request_id`（`string`）。`run_code` か `submit_code` で渡した ID。
- 戻り値: `null`。止めた結果は `run_finished` で知らせる。実行・テスト実行では `run.status` が `stopped`、提出では `submit.status` が `stopped` になる。
- 動いていない実行・提出の ID（既に終わったもの、無いもの）を渡したときは、何もせずに `null` を返す。
- 提出には停止ボタンが無い（[コードの提出（submit）](../../exercise-runner/submit.md)）。提出の ID を渡すのは、演習画面を離れるときだけである。
- アプリを終了したときは、バックエンドが自分で止める（[コードの実行（run）](../../exercise-runner/run.md)「停止」）。

## submit_code

演習画面の「提出」で呼ぶ（[コードの提出（submit）](../../exercise-runner/submit.md)）。

- 引数:

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | この提出の ID |
| `lesson_id` | `string` | レッスン ID（生成問題は `gen-{generation_id}`） |
| `files` | `{ [path: string]: string }` | エディタ上の全ファイル |

- 戻り値: `null`。提出を始めたら返す。ビルドの出力は `run_output`、判定の進み具合は `submit_progress`、判定結果は `run_finished` で知らせる。
- 始める前に、`files` を下書きとして保存する（`run_code` と同じ）。
- エラー: `run_code` と同じ（C の演習のエラーも同じ。`mode` の条件は無い）。

## イベント

**`run_started`**

`run_code` の `mode` が `main` の実行で、学習者のプログラムを起動したときに 1 回送る（[コードの実行（run）](../../exercise-runner/run.md)「手順」の 3、C の演習では [C のコードの実行（run）](../../exercise-runner/c-run.md)「手順」の 3）。演習画面は、受け取ったら標準入力の入力欄を有効にする（同じく「対話的な標準入力」）。

```json
{ "request_id": "3f0c…" }
```

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | 実行の ID |

- バックエンドは、`send_stdin`・`close_stdin` を受け付けるようにしてから送る。
- ビルドの出力の `run_output` より後、`run_finished` より前に届く。学習者のプログラムの出力の `run_output` との前後は決めない。
- テスト実行（標準入力は起動した直後に閉じる）と提出では送らない。プログラムを起動しなかったとき（ビルドの失敗、`crates_unavailable`、起動する前に止めたとき、アプリの側の失敗）も送らない。

**`run_output`**

実行・テスト実行・提出の出力（[コードの実行（run）](../../exercise-runner/run.md)「出力の送り方」）。

```json
{
  "request_id": "3f0c…",
  "chunks": [
    { "stream": "stdout", "text": "数を入力してください: " },
    { "stream": "system", "text": "実行時間の制限（60 秒）を超えたため停止しました。" }
  ]
}
```

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | 実行・提出の ID |
| `chunks` | `object[]` | 出力の断片。受け取った順に並べる |
| `chunks[].stream` | `"stdout" \| "stderr" \| "system"` | 断片の種類 |
| `chunks[].text` | `string` | 断片の文字列。ANSI エスケープを含むことがある |

- `stdout`・`stderr` は、学習者のプログラム（テスト実行では `cargo test`）の標準出力・標準エラー出力とする。
- ビルドの出力（[コードの実行（run）](../../exercise-runner/run.md)「ビルドの出力の表示」の、コンパイラの診断と cargo 自身の標準エラー出力。C の演習では [C の演習の作業領域とビルド](../../exercise-runner/c-project-assembly.md)「ビルドの出力の表示」の、`clang` の出力）は、`stderr` として送る。
- `system` は、アプリが出す知らせ（タイムアウトの旨、出力量の上限で停止した旨、「外部クレートを準備し直しています」、隠しテストの側でエラーが出たときの案内、C の演習でシグナルで終わったときの 1 行（[C のコードの実行（run）](../../exercise-runner/c-run.md)「シグナルで終わったとき」）など）とする。文は、バックエンドが日本語で作る。
- 学習者が入力した標準入力は送らない。コンソールへの表示は、フロントエンドが `send_stdin` で送った文字列から行う。
- 提出では、プログラムの出力を送らない（[コードの提出（submit）](../../exercise-runner/submit.md)）。

**`submit_progress`**

提出の判定の進み具合（[コードの提出（submit）](../../exercise-runner/submit.md)）。段階が変わったときと、テストケースが 1 件終わるたびに送る。

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | 提出の ID |
| `phase` | `"build" \| "judge"` | `build` はビルド中、`judge` はテストケースの実行中 |
| `completed` | `number`（整数） | 結果が決まったテストケースの数。`build` では 0 |
| `total` | `number \| null` | 全テストケースの数。決まる前（`build` と、`test` 方式で隠しテストの名前を得る前）は `null` |

**`run_finished`**

実行・テスト実行・提出が終わったときに 1 回送る。

| 項目 | 型 | 内容 |
|---|---|---|
| `request_id` | `string` | 実行・提出の ID |
| `kind` | `"run" \| "submit"` | `run_code` の実行なら `run`、`submit_code` の提出なら `submit` |
| `finished_at` | `string` | 終わった日時 |
| `error` | [コマンドとイベントの共通の決まり](conventions.md)「エラー」の形 `\| null` | アプリの側の失敗（`project/`・`c-project/` を組み立てられなかった、cargo・`clang` を起動できなかったなど）で、結果の状態を決められずに終わったときのエラー。このとき `run`・`submit`・`progress` は `null` にする。ほかでは `null` |
| `run` | `object \| null` | `kind` が `run` のときの結果。`submit` と、`error` があるときは `null` |
| `run.mode` | `"main" \| "test"` | `run_code` で渡した `mode` |
| `run.status` | `string` | 結果の状態（[コードの実行（run）](../../exercise-runner/run.md)「結果の状態」） |
| `run.exit_code` | `number \| null` | 学習者のプログラム（テスト実行では `cargo test`）の終了コード。起動していないときと、シグナルで終わったときは `null` |
| `submit` | [コードの提出（submit）](../../exercise-runner/submit.md)「判定結果」の形 `\| null` | `kind` が `submit` のときの判定結果（`status`・`passed`・`total`・`cases[]`）。C の演習では、`cases[].status` に `memory_error` があり、その `detail` はオブジェクトになる（[C のコードの提出（submit）](../../exercise-runner/c-submit.md)「判定結果」）。`run` と、`error` があるときは `null` |
| `progress` | [進捗（progress.json）](../../lesson-flow/progress.md) の `lessons` の 1 件 `\| null` | 提出で正解してレッスンを完了にしたときの、更新した後の進捗（[レッスンの完了](../../lesson-flow/completion.md)）。生成問題とそれ以外のときは `null` |

- 試行ログへの記録は [実行・提出の試行ログへの記録](../../exercise-runner/attempt-log.md) に従う。`run_finished` は、記録を終えてから送る。
