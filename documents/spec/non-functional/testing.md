---
type: Specification
title: "テストの方針"
description: "テストの種類と道具（`cargo test`、Vitest、秘密の情報の検査のテスト、教材チェックツール、人が操作して確かめること）、自動テストで確かめるもの（実物の `clang` とメモリリークの検査のライブラリを使う C の演習のテストを macOS のときだけ実行することを含む）、決まった出力を返す偽の `claude`・`cargo`・`clang` の作り方と使い方、受け入れ条件の確かめ方の区分、AI の出力に左右される条件の扱い、画面を自動で操作するテスト（E2E）を行わないこと、テストを実行するとき（コミット前、push の前の秘密の情報の検査、教材やデザインシステムを変えたとき、仕様の文書を移したり名前や小見出しを変えたとき、本番ビルド）、CI で実行するもの（秘密の情報の検査とそのテスト、`npm run check`、教材チェックツール、デザインシステムの検査）と、CI の秘密の情報の検査の限界と、CI の実行環境を定める。"
---

## テストの種類と道具

| 種類 | 対象 | 道具 |
|---|---|---|
| バックエンドの自動テスト | `src-tauri/` | `cargo test` |
| フロントエンドの自動テスト | `src/` の関数・フック・部品 | Vitest。React の部品は React Testing Library と jsdom で描画する |
| 開発用ツールの自動テスト | `tools/` の Node.js で動くツール（教材チェックツール、音声を作るツール） | Vitest |
| 秘密の情報の検査のテスト | `tools/check-secrets.py`（[開発環境と依存の版](../architecture/development-environment.md)「秘密の情報の検査」） | Python の `unittest`（`tools/tests/test_check_secrets.py`）。一時ディレクトリに作った git のリポジトリで、本物の git と gitleaks を使う。gitleaks が無いときと版が違うときは、飛ばさずに失敗する |
| 教材のチェック | `content/` | 教材チェックツール（[教材チェックツール](../content-guidelines/check-tool.md)） |
| 人が操作して確かめること | アプリの画面 | 開発時の起動か本番ビルドのアプリ（[開発環境と依存の版](../architecture/development-environment.md)「開発で使うコマンド」） |

- 画面を自動で操作するテスト（E2E）は行わない。
- 道具の版は [開発環境と依存の版](../architecture/development-environment.md)「主な依存の版」に従う。
- テストのコードは、Rust は対象のモジュールの `#[cfg(test)]` と `src-tauri/tests/` に、TypeScript は対象のファイルと同じディレクトリの `{名前}.test.ts`・`{名前}.test.tsx` に置く。

## 自動テストで確かめるもの

- バックエンドのロジックは、自動テストで確かめることを基本にする。次のものを含む。
  - 判定（`stdout` 方式・`test` 方式、テスト結果の読み取り、C の演習のメモリの検査と `memory_error`）と、学習者のプログラムの実行（制限時間、出力の上限、プロセスグループの停止、標準入力）（[コード実行・判定](../exercise-runner/index.md)）
  - 集計と計算（進捗、推奨レッスン、学習記録のアプリ管理フィールド、`base_level` など）
  - 検証（AI の出力の検証、教材の定義の読み込み）
  - ファイルの読み書き（[データの保存場所](../architecture/data-locations.md)「書き込みの規則」、[データの互換性](data-compatibility.md) の形式の変換と退避）
  - ジョブキュー（実行の順序、リトライ、`failed` への移り変わり）（[Claude Code 連携](../claude-integration/index.md)）
  - 外部ツールの検出と、`claude` の起動の引数・環境変数、出力の読み取り、エラーの分類（[外部ツール（cargo・claude）への依存](../architecture/external-tools.md)、[C のツール（clang）への依存](../architecture/c-toolchain.md)、[claude -p の呼び出しの共通ルール](../claude-integration/invocation.md)、[AI 呼び出しのエラーの分類と表示](../claude-integration/errors.md)）
- バックエンドのテストは、`{app_data_dir}` と `{app_resources}` の代わりに、テストごとの一時ディレクトリを使う。開発者の実際の `{app_data_dir}` を読み書きしない。
- バックエンドのテストは、アプリのウィンドウを開かずに実行できるようにする。ロジックは Tauri の実行時に頼らずに呼べるようにし、Tauri のコマンドを確かめるときは Tauri のモックの実行環境（`tauri::test`）を使う。
- 学習者のプログラムのビルドと実行を確かめるテストは、実物の `cargo` を使う。
- C の演習のビルド・実行・判定を確かめるテストは、実物の `clang` とメモリリークの検査のライブラリ（[C のコードの提出（submit）](../exercise-runner/c-submit.md)「メモリリークの検査」）を使い、macOS のときだけ実行する。
- フロントエンドの自動テストでは、画面の部品が使う計算と状態の扱い（動画のタイムラインと合図の計算、キー操作の受け方、音声の URL の作り方など）を確かめる。バックエンドのコマンドの呼び出しは、テストの中で決まった値を返すものに置き換える。

## 偽の claude と偽の cargo

- AI の出力を受けた後のアプリの動作（出力の検証、ファイルへの書き込み、アプリが計算する値、エラーの扱い、ジョブの順序）は、決まった出力を返す偽の `claude` を使う自動テストで確かめる。
- 偽の `claude` は、テスト用の実行ファイルにする。perl の組み込みの機能だけで書いたスクリプト（`/usr/bin/perl`）にする。次のように動く。
  - `--version` に成功する。用意した失敗（終了コード、空の出力、応答しないこと）も返せる。
  - `-p` の呼び出しには、[claude -p の呼び出しの共通ルール](../claude-integration/invocation.md)「出力形式」の形で、用意した出力を返す。出力を 2 回に分けて書くこともできる。用意した失敗（エラーの出力、終了コード、応答しないこと、SIGTERM を無視すること、同じプロセスグループに終わらない子のプロセスを作ること）も返せる。
  - 受け取った引数・環境変数・標準入力を、ファイルに書き残す。`-p` の呼び出しでは、自分のプロセスの番号とプロセスグループ、作業ディレクトリ（中身と、どのディレクトリか）も書き残す。
- 偽の `cargo`・`rustc` は、sh のスクリプトにする。
- 偽の `clang` も、sh のスクリプトにする。`--version` に、用意した出力を返すか、用意した失敗（終了コード、空の出力、応答しないこと）を返す。`clang` が見つからないとき・使えないときの動作（検出の結果、`clang_unavailable`、そのときも Rust の演習を実行・提出できること）は、偽の `clang` を使う自動テストで確かめる。
- テストでは、偽のツールの原本を指すシンボリックリンクを、テストごとの一時ディレクトリに置いて使う。
- 偽の `claude` がどの出力を返すかは、実行ファイルのパスか、実行ファイルと同じディレクトリに置いたファイルで決める。環境変数では決めない。偽の `claude` が受け取る環境変数も、実物と同じく [claude -p の呼び出しの共通ルール](../claude-integration/invocation.md)「環境変数」のものだけになる。
- 実物の `cargo` で起こしにくい状態（`rustc` のバージョンが足りない、など）は、偽の `cargo`・`rustc` を使う自動テストで確かめる。
- バックエンドのテストは、偽のツールのパスを、検出の結果として直接与える。アプリを起動して人が確かめるときは、`settings.json` の `tool_paths`（[設定画面と settings.json](../screens/settings-screen.md)）に偽のツールのパスを書いて使える。
- 偽のツールは、テストのコードと一緒に `src-tauri/tests/` の下に置き、アプリに同梱しない。

## 受け入れ条件の確かめ方

各機能の受け入れ条件（`acceptance.md`）は、次のどれかの区分で確かめる。自動テストで確かめられる条件は、自動テストで確かめる。

| 区分 | 確かめるもの |
|---|---|
| 自動テスト | ロジックで決まる条件。実物の `cargo` を使うものと、実物の `clang` とメモリリークの検査のライブラリを使うものを含む |
| 偽のツールを使う自動テスト | AI の出力を受けた後のアプリの動作。ツールが見つからない・失敗する・バージョンが足りないときの動作 |
| 教材チェックツール | 「教材チェックツールでエラーになる」と書いた条件と、教材の内容についての条件（[教材チェックツール](../content-guidelines/check-tool.md)） |
| 人が操作して確かめる | 上のどれでも確かめられない条件（下の一覧） |

- 各受け入れ条件をどの区分で確かめるかは、その機能を実装するときに決める。
- 教材チェックツールのチェック項目は、エラーになる教材とならない教材を用意した、開発用ツールの自動テストで確かめる。
- 音声を作るツールの条件（[教材データ形式の受け入れ条件](../content-format/acceptance.md) の、音声の作り直しについての条件）は、作者が、作者の環境の VOICEPEAK と ffmpeg を使ってツールを実行し、確かめる。ツールのうち VOICEPEAK を使わない処理（作り直す文の選び方、`durations.json` の更新など）は、開発用ツールの自動テストで確かめてよい。

人が操作して確かめるもの:

- 画面の表示、見た目、操作感。作者が、実装された画面を見て確かめる。
- 動画の再生（音声と絵の同期、シーク、テーマの切り替え）と、エディタでの日本語の入力。
- Finder から起動したときのツールの検出など、アプリを起動する方法で変わる条件。
- 本番ビルドでしか確かめられない条件（[非機能要件の受け入れ条件](acceptance.md) の CSP の条件）と、[性能の目安](performance.md)。
- 実物の `claude` を使う条件（「AI の出力に左右される条件」）。

## AI の出力に左右される条件

- 実物の `claude` を呼ぶ確認は、自動テストに入れない。作者が、実物の `claude` を使ってアプリを操作し、確かめる。AI の回答の中身についての条件（完成コードを返さないこと、直前の会話や再生位置を踏まえて答えることなど）と、学習者の Claude Code の設定が AI の呼び出しに影響しないことは、ここで確かめる。
- 同じ条件のうち、アプリの動作で決まる部分は、偽の `claude` を使う自動テストで確かめる。たとえば、AI がアプリの計算と異なる値を出力する場合、検証に失敗する出力を返す場合、`claude` が失敗する場合を、偽の `claude` の出力として用意する。

## 実行するとき

- コミットする前に、`npm run check`（[開発環境と依存の版](../architecture/development-environment.md)「開発で使うコマンド」）を実行し、通ることを確かめる。AI エージェントがコミットするときも同じにする。
- push する前に、`npm run secrets:check`（[開発環境と依存の版](../architecture/development-environment.md)「秘密の情報の検査」）を実行し、通ることを確かめる。AI エージェントが push するときも同じにする。実装のループは、機械のチェックと push の直前に実行する（[実装のループ](../../conventions/implementation-loop.md)「機械のチェック」）。
- `tools/check-secrets.py`・`.gitleaks.toml`・gitleaks の版を変えたときは、`python3 -I tools/tests/test_check_secrets.py` を実行する。
- 教材を追加・変更したときは、教材チェックツールを実行する（[教材チェックツール](../content-guidelines/check-tool.md)）。
- デザインシステムか画面デザインを変えたときは、`python3 tools/design-system.py check` を実行する（[デザインシステムのファイルと変更の手順](../screens/design-system-files.md)「検査」、[画面デザインのファイルと変更の手順](../screens/ui-design-files.md)「検査」）。仕様の文書（`documents/spec/`）を移したり、名前や小見出しを変えたりしたときも実行する。デザインシステムの README などからの、仕様への参照を確かめるためである。
- 本番ビルドを人が操作して確かめることは、次のときに行う。
  - 依存の版を上げたとき（[開発環境と依存の版](../architecture/development-environment.md)「版を上げるとき」）
  - CSP の読み込み先に関わる変更をしたとき（[セキュリティとプライバシー](security-privacy.md)「アプリ自体」）
  - 機能の受け入れ条件を満たしたかを判断するとき
- 実物の `claude` を使う確認は、AI に渡す入力（指示文、会話履歴、画面の状態の渡し方）と、`claude` の起動のしかたを変えたときと、機能の受け入れ条件を満たしたかを判断するときに行う。

## CI

- リポジトリへの push と pull request のたびに、GitHub Actions で次のものを実行する。
  - 秘密の情報の検査（`python3 -I tools/check-secrets.py --all`。[開発環境と依存の版](../architecture/development-environment.md)「秘密の情報の検査」）と、そのスクリプトのテスト（`python3 -I tools/tests/test_check_secrets.py`）。ほかのものとは別のジョブ（`secrets`）で実行する。コミットの履歴の全部を取ってから（`actions/checkout` の `fetch-depth: 0`）、取ったすべての参照から届くコミットを確かめる。gitleaks は、固定した版の公式のリリースの Linux 用のファイル（`gitleaks_{版}_linux_x64.tar.gz`）を取り、ワークフローに書いた SHA-256 と合うことを確かめてから使う。サードパーティの Action は使わない
  - `npm ci` の後の `npm run check`。`cargo` のコマンドは、`src-tauri/Cargo.lock` のとおりの依存で実行する（`--locked`）
  - 教材チェックツール（[教材チェックツール](../content-guidelines/check-tool.md)）
  - デザインシステムと画面デザインの検査（`python3 tools/design-system.py check`。[デザインシステムのファイルと変更の手順](../screens/design-system-files.md)「検査」、[画面デザインのファイルと変更の手順](../screens/ui-design-files.md)「検査」）
- CI の秘密の情報の検査には、次の限界がある。push の前の検査（上の「実行するとき」）と、GitHub の設定（[リポジトリ構成](../architecture/repository.md)「GitHub の設定」の、fork からの pull request のワークフローの承認と Interaction limits）で補う。
  - 衝突のある pull request では、`pull_request` のワークフローが走らない。
  - `pull_request` のワークフローは、pull request のマージのコミットで動くので、その pull request で変えたワークフロー・スクリプト・設定が使われる。fork からの pull request も同じである。
  - CI が見つけるのは push の後なので、公開リポジトリでは、見つけた時点で、すでに公開されている。
- CI の実行環境は GitHub Actions の ubuntu-latest である。CI の実行環境で、対応環境（[対応環境](environments.md)）ではない。Tauri が Linux でのビルドに要るライブラリを入れて実行する。
- macOS に固有の動作を確かめるテストは、macOS のときだけ実行するようにし、CI では実行しない。CI の結果を、対応環境で動くことの確かめにしない。
- 実物の `clang` とメモリリークの検査のライブラリを使う C の演習のテストも、macOS に固有の動作を確かめるテストとして扱い、CI では実行しない。教材チェックツールの C のレッスンの「模範解答」「初期コード」の判定も、macOS でないときは行わず、行わなかったことを警告として表示する（[教材チェックツール](../content-guidelines/check-tool.md)）。これらは、作者の Mac で行うコミット前の確認と、実装のループの機械のチェック（[実装のループ](../../conventions/implementation-loop.md)「機械のチェック」）で確かめる。
- 作者の Mac では通り、CI の実行環境でだけ失敗するビルドやテストがあるときは、アプリのコードを Linux に合わせて直さない。そのテストを macOS のときだけ実行するようにするか、そのビルドやテストを CI で実行するものから外す。
- CI では、本番ビルドと、実物の `claude` と VOICEPEAK を使う確認を行わない。
- CI が失敗したら、次の変更より先に直す。
