# アーキテクチャ

* [アーキテクチャの目的とスコープ](scope.md) - アーキテクチャの仕様が扱う範囲（技術構成、役割分担、コマンドとイベント、外部ツールへの依存、データの保存場所、リポジトリ構成、開発環境と依存の版、Tauri の権限）を定める。
* [アプリの技術構成](tech-stack.md) - 層ごとの採用技術（Tauri 2、React + TypeScript、Monaco Editor、react-markdown、Remotion、Rust、cargo、clang、Claude Code、VOICEPEAK）と、アプリの構成図を定める。
* [フロントエンドとバックエンドの役割分担](responsibilities.md) - フロントエンドがファイルシステムや子プロセスに直接触れないこと、動画のコードをフロントエンドのビルドに含め、動画の音声をアプリ同梱データからアセットプロトコルで読むことなど両者の役割分担と、コマンド・イベントの名前と担当仕様、入力・出力の形を定める文書（`commands/`）の参照先を定める。
* [コマンドとイベントの入力・出力](commands/index.md) - フロントエンドとバックエンドの間のコマンドとイベントについて、共通の決まり（名前、JSON の書き方、エラーの形、要求を見分ける ID）と、分類ごとの入力・出力の形を定める。
* [外部ツール（cargo・claude）への依存](external-tools.md) - cargo と claude の検出方法と必要条件、見つからない場合の動作と導入手順の URL、rustc のバージョンが足りない場合の案内、許可クレートの初回準備を定める。
* [C のツール（clang）への依存](c-toolchain.md) - C の演習のビルドに使う `clang` の検出方法（設定の `tool_paths.clang` → `PATH` → `/usr/bin`）と必要条件とバージョンの取り出し方、`clang` が見つからない場合の動作と Command Line Tools の導入の案内（`xcode-select --install` を画面に表示し、URL を開かないこと）、`clang` を同梱せず自動で導入しないことを定める。
* [データの保存場所](data-locations.md) - 学習者データ（`{app_data_dir}`）とアプリ同梱データ（`{app_resources}`）のディレクトリ構成と、ファイルの書き込みの規則を定める。
* [リポジトリ構成](repository.md) - リポジトリ直下のディレクトリ（`.github/workflows/`、`documents/`、`design-system/`、`content/`、`prompts/`、`runner/`、`tools/`、`src/`、`src-tauri/`）とファイル（`README.md`、`.gitleaks.toml`、`.gitignore`、`skills-lock.json`、`package.json`、`package-lock.json`）の役割と、git に入れないもの（Remotion のスキルなど）と、リポジトリを GitHub で公開することと、そのための GitHub の設定（外部の人の Issue・pull request を受け付けない設定と、その期限を含む）を定める。
* [開発環境と依存の版](development-environment.md) - アプリの開発に要るもの（Node.js、npm、Rust ツールチェーン、Xcode Command Line Tools、gitleaks）、パッケージマネージャー（npm）、主な依存の版と版の固定のしかた（`package.json`・`package-lock.json`・`Cargo.lock`）、版を上げるときに確かめること、アプリの識別子（`com.progrust.desktop`）と開発時の識別子、lint と整形の道具、秘密の情報の検査（gitleaks の版の固定、`npm run secrets:check` が確かめるものと確かめないもの、自己点検、版を変えるとき）、git に入れないエージェントのスキル（Remotion のスキル）を `skills-lock.json` から入れ直す手順、開発時の起動・本番ビルドなどの npm のコマンドの名前を定める。
* [Tauri の権限](tauri-permissions.md) - フロントエンドに許可する Tauri の権限（capabilities）の範囲と、外部 URL を開ける範囲、アセットプロトコルで読める範囲、外部サイトをアプリ内で表示しないことを定める。
* [アーキテクチャの受け入れ条件](acceptance.md) - アーキテクチャの仕様を満たしたと判断する条件を定める。
* [アーキテクチャの未決事項](open-questions.md) - アーキテクチャの仕様でまだ決まっていない事項（読み込めないファイルを退避したときの通知の出し方）を記す。
