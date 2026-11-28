---
type: Decision
title: "ADR 0266"
description: "C の演習を学習者の Mac の clang でビルドし、実行はメモリの検査なしで動かし、提出は AddressSanitizer・UndefinedBehaviorSanitizer と、確保と解放を数えるライブラリによるメモリの検査つきで判定する"
---

- 日付: 2026-11-28
- 関連仕様: [C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)、[C のコードの実行（run）](../spec/exercise-runner/c-run.md)、[C の演習の作業領域とビルド](../spec/exercise-runner/c-project-assembly.md)、[C のツール（clang）への依存](../spec/architecture/c-toolchain.md)、[演習のファイル形式](../spec/content-format/exercise.md)「C の演習」、[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)「C のレッスン」、[実行・判定の制限値](../spec/exercise-runner/limits.md)、[コード実行・判定の概要](../spec/exercise-runner/overview.md)、[コード実行・判定の未決事項](../spec/exercise-runner/open-questions.md)、[実行のコマンドとイベント](../spec/architecture/commands/run.md)、[環境のコマンドとイベント](../spec/architecture/commands/environment.md)、[コマンドとイベントの共通の決まり](../spec/architecture/commands/conventions.md)、[アーキテクチャの未決事項](../spec/architecture/open-questions.md)、[演習画面](../spec/screens/exercise-screen.md)、[初回起動画面](../spec/screens/first-launch.md)、[設定画面と settings.json](../spec/screens/settings-screen.md)、[演習コードの下書き](../spec/lesson-flow/drafts.md)、[対応環境](../spec/non-functional/environments.md)、[テストの方針](../spec/non-functional/testing.md)、[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)、[教材チェックツール](../spec/content-guidelines/check-tool.md)
- 置き換え: なし

## 背景

カリキュラムの最初に C の章を置き、C のメモリ管理の難しさを演習で体験させることにした（[ADR 0262](0262-teach-c-chapters-before-rust.md)）。演習のコードは、学習者の環境の `cargo` でビルドして実行していて、C のコードを動かす仕組みは無かった。

Claude は、2026-11-28 に、作者の Mac（Apple clang 21、macOS 27、Apple シリコン）で次のことを確かめた。

- メモリの検査なしでビルドすると、解放後の使用、範囲外アクセス、戻った関数のローカル変数の使用は、エラーなしで終了コード 0 で終わり、誤った値を出力した。二重解放は異常終了した。
- `-fsanitize=address` を付けてビルドすると、解放後の使用、二重解放、範囲外アクセスを検出して異常終了した。戻った関数のローカル変数の使用は、既定では検出せず、環境変数 `ASAN_OPTIONS` に `detect_stack_use_after_return=1` を指定すると検出した。
- `-fsanitize=undefined` は、符号付き整数のあふれを報告したが、既定では実行を続けて終了コード 0 で終わった。`-fno-sanitize-recover=all` を付けると異常終了した。
- `ASAN_OPTIONS` の `log_path` を指定すると、どちらの検査の報告も、標準エラー出力ではなく、`{log_path}.{プロセス ID}` という名前のファイルに書かれた。`log_path` に、空白を含む絶対パスを指定すると、`ASAN_OPTIONS` の読み取りに失敗し、報告は書かれなかった。相対パスなら書かれた。
- AddressSanitizer のメモリリークの検出は、この clang では、この環境では使えないと表示された。
- macOS 付属の `leaks --atExit -- {実行ファイル}` は、検査なしのビルドに対して動き、リークの件数を報告した。ただし、解放していない領域があるのに、0 件と報告することがあった。関数の中で確保して解放せずに戻ったプログラムを、0 件と報告した。実行・判定の仕様を書いた別の会話の Claude の確認では、逆に、`main` の変数に残したまま終わったプログラムを 0 件と報告し、関数の中で失った領域は報告した。どちらの確認でも、見逃しがあった。プログラムの終了コードは分からず、プログラムの標準出力は `leaks` の出力に混ざった。1 回に約 0.6 秒かかった。
- 確保と解放を数える動的ライブラリを試作し、環境変数 `DYLD_INSERT_LIBRARIES` で、検査なしのビルドに差し込んだ。ライブラリは、`malloc`・`calloc`・`realloc`・`free` を置き換え、学習者の実行ファイルのコードから呼ばれた確保だけを記録し、終了時に、解放されていない領域の件数を書き出す。試したすべての場合（`main` の変数に残したままの未解放、関数の中で失った領域、連結リストの未解放、すべて解放したプログラム、`strdup` と `realloc` を使ってすべて解放したプログラム、標準入力をパイプで渡す場合）で、正しい件数を出した。プログラムの標準出力と終了コードは変わらなかった。`-fsanitize=address` のビルドに差し込むと、数えられなかった。
- 初期化していないヒープの領域の読み取りは、どの検査でも報告されなかった。
- clang は、`-Wall -Wextra` で、ローカル変数のアドレスをそのまま返す関数と、初期化していない変数の使用を、単純な場合だけ警告した。

Claude は、「実行」ボタンでメモリの検査をするかどうかについて、ユーザーに次の 3 つの案を示した。提出は、どの案でも検査つきで判定し、誤りの種類と行番号を表示する。

- 実行は検査なし、提出で検査する（Claude が勧めた）。
- ボタンを 2 つにする（「実行」と「検査つきで実行」）。
- 実行も常に検査つきにする。

ユーザーは、実行は検査なし、提出で検査する案を選んだ。

## 決定

- C の演習の実行は、メモリの検査なしで動かす。提出は、メモリの検査つきで判定し、誤りの種類と行を表示する（[C のコードの実行（run）](../spec/exercise-runner/c-run.md)、[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)）。
  - 理由: ユーザーが選んだ。Claude が勧めた理由は、実行で C のそのままの動きを見せると、動いたように見えたのに提出で誤りが見つかる体験になり、Rust のコンパイラが先に止めることのありがたさにつながることである。
- 次の細部は Claude が決めた。
  - C の演習は、学習者の Mac の `clang` でビルドする。`clang` は同梱せず、アプリから導入もしない（[C のツール（clang）への依存](../spec/architecture/c-toolchain.md)）。理由: Rust ツールチェーンと同じ考え方である（[ADR 0016](0016-do-not-install-rust.md)）。Rust のビルドには Command Line Tools のリンカが要るので、Rust を動かせる Mac には `clang` がある。
  - `clang` の導入は、URL を開かず、画面にコマンド（`xcode-select --install`）を表示して案内する（同上）。理由: アプリが開く外部の URL を増やさない（[ADR 0240](0240-install-guide-urls-and-opener-scope.md)）。導入はこのコマンド 1 つで済む。
  - C の演習は、1 ファイル（`main.c`）で、判定は標準出力の比較だけにする（[演習のファイル形式](../spec/content-format/exercise.md)「C の演習」、[レッスン定義の形式（lesson.json）](../spec/content-format/lesson-json.md)「C のレッスン」）。理由: 複数ファイルにするにはヘッダーファイルの作り方を教える必要があり、メモリの扱いから離れる（[ADR 0263](0263-c-chapters-scope-three-chapters.md)）。隠しテスト（[ADR 0009](0009-embed-hidden-tests-as-module.md)）は Rust のテストの仕組みで、C には使えない。
  - メモリの誤りの検査には AddressSanitizer と UndefinedBehaviorSanitizer を、メモリリークの検査には、確保と解放を数えるライブラリ（メモリリークの検査のライブラリ）を使う。提出では、検査つきと検査なしの 2 つの実行ファイルをビルドし、メモリリークの検査は、ほかの条件を通ったテストケースだけに、検査なしの実行ファイルで行う（[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)、[C の演習の作業領域とビルド](../spec/exercise-runner/c-project-assembly.md)）。理由: 上の「背景」で確かめたとおり、この組み合わせで、C の章で扱う誤り（[ADR 0263](0263-c-chapters-scope-three-chapters.md)）を見つけられる。`leaks` には見逃しがあり、解放を書き忘れた解答が正解になりうる。数えるライブラリは、検査つきのビルドでは数えられないので、判定とは別に起動する。
  - メモリリークの `detail` には、確保した行を入れない（同上）。理由: 数えるライブラリからは、確保した行が分からない。
  - 提出のビルドは、検査なし、検査つきの順に行い、検査つきのビルドの出力は、失敗したときだけ表示する（[C の演習の作業領域とビルド](../spec/exercise-runner/c-project-assembly.md)）。理由: 同じ警告が 2 回表示されないようにする。
  - 検査の報告は、作業領域の中の相対パスに書き出させる（同上）。理由: 上の「背景」のとおり、空白を含む絶対パスでは書き出されなかった。
  - C の演習の実行がシグナルで終わったときは、シグナルの名前と意味を 1 行で表示する（[C のコードの実行（run）](../spec/exercise-runner/c-run.md)）。理由: 実行はメモリの検査なしなので、異常終了の手がかりがそれだけになる。
  - 失敗したテストケースの名前の一覧でも、`memory_error` のものには、誤りの種類の名前と行を添える（[演習画面](../spec/screens/exercise-screen.md)）。理由: 詳細を表示するのは最初に失敗したテストケースだけなので、それがメモリの誤りでないときにも、種類が見えるようにする。
  - `clang` の版は、必要条件にしない（[C のツール（clang）への依存](../spec/architecture/c-toolchain.md)）。
  - `settings.json` に `tool_paths.clang` を足し、`schema_version` を 2 に上げて、版 1 から変換する（[設定画面と settings.json](../spec/screens/settings-screen.md)）。理由: 版を上げずにキーを足すと、いまの設定ファイルが、形に合わないものとして退避され、設定が初期値に戻る。
  - メモリの検査が誤りを報告したテストケースの結果を、新しい `memory_error` にし、`detail` に誤りの種類・行・報告を入れる（[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)、[実行のコマンドとイベント](../spec/architecture/commands/run.md)）。理由: `runtime_error` に含めると、画面で誤りの種類と行を見出しにできない。メモリの誤りを、ほかの異常終了と区別して見せることが、C の章の目的に合う。
  - 模範解答も、メモリの検査で誤りが報告されないことを、教材チェックツールで検証する（[教材チェックツール](../spec/content-guidelines/check-tool.md)）。
  - C の演習には「テスト」ボタンを置かない。[ADR 0112](0112-run-learner-tests-with-test-button.md) の「決定」の「どの演習でも表示する」を、Rust の演習に限る、に変える（[演習画面](../spec/screens/exercise-screen.md)）。理由: テスト実行は `cargo test` を動かす機能で、C には当たるものが無い。
  - 実行・提出を同時に 1 つだけ行う制約は、C と Rust で共通にする（[コード実行・判定の概要](../spec/exercise-runner/overview.md)）。
  - C のコードも、サンドボックスに入れずに実行する（[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)）。理由: Rust の演習と同じ考え方である（[ADR 0014](0014-run-learner-code-without-sandbox.md)）。
  - 確かめていないこと（検査の報告から種類と行を取り出す方法、メモリリークの検査のライブラリの作り方と置き方、Command Line Tools が無い Mac での `clang` の検出、ほかの環境での動作、CI での扱い）は、未決事項に残した（[コード実行・判定の未決事項](../spec/exercise-runner/open-questions.md)、[アーキテクチャの未決事項](../spec/architecture/open-questions.md)）。実装計画を作る前に決める。

## 検討した他の案

- 案: 実行も常にメモリの検査つきにする
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、C では誤りがあっても黙って動いてしまう、という体験ができなくなることである
- 案: 「実行」と「検査つきで実行」の 2 つのボタンを置く
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、C の演習だけボタンが 1 つ増えることである
- 案: メモリの誤りを `runtime_error` に含める
  - 採用しなかった理由: 上の「決定」の `memory_error` の理由のとおり
- 案: `clang` をアプリに同梱する
  - 採用しなかった理由: 配布サイズが大きく増える。Rust ツールチェーンを同梱しない方針（[ADR 0016](0016-do-not-install-rust.md)）とも異なる
- 案: メモリリークを、macOS 付属の `leaks` で検査する（Claude の最初の設計）
  - 採用しなかった理由: 上の「背景」のとおり、解放していない領域を 0 件と報告することがあった
- 案: メモリリークを、学習者のコードのビルドに、`malloc` と `free` を置き換えるコードを混ぜて数える
  - 採用しなかった理由: 学習者のコードのビルドが、手元で `clang` を使うときと違うものになる。実行のときに差し込む方法なら、ビルドは変わらない
- 案: メモリリークを検査しない
  - 採用しなかった理由: メモリリークは C の章で扱う誤りの 1 つ（c03-05）で、検査しないと、解放を書かない解答が正解になる

## 影響

- C の演習の提出は、ビルドが 2 回になり、通ったテストケースごとに、メモリリークの検査のための起動が 1 回加わる。
- メモリリークの検査のライブラリを、アプリの側で用意する必要がある。試作で確かめただけで、作り方と置き方は決まっていない。
- 初期化していない領域の読み取りは、メモリの検査では報告されない。出力が期待値と違うことで不正解になるが、何が誤りかは示されない。c03-01 で、検査が見つけないものとして教える。
- メモリの検査は、macOS の `clang` と、macOS の動的ライブラリを差し込む仕組み（`DYLD_INSERT_LIBRARIES`）に頼る。CI の実行環境は ubuntu-latest なので、C の判定のテストと教材の判定を CI でどう動かすかは、未決事項である（[テストの方針](../spec/non-functional/testing.md)）。
- 学習者の環境に必要なものに、Command Line Tools が加わった（[対応環境](../spec/non-functional/environments.md)）。
