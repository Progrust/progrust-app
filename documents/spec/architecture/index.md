# アーキテクチャ

* [アーキテクチャの目的とスコープ](scope.md) - アーキテクチャの仕様が扱う範囲（技術構成、役割分担、外部ツールへの依存、データの保存場所）を定める。
* [アプリの技術構成](tech-stack.md) - 層ごとの採用技術（Tauri 2、React + TypeScript、Monaco Editor、Remotion、Rust、cargo、Claude Code、VOICEPEAK）と、アプリの構成図を定める。
* [フロントエンドとバックエンドの役割分担](responsibilities.md) - フロントエンドがファイルシステムや子プロセスに直接触れないことなど両者の役割分担と、コマンド・イベントの名前と担当仕様を定める。
* [外部ツール（cargo・claude）への依存](external-tools.md) - cargo と claude の検出方法と必要条件、見つからない場合の動作、許可クレートの初回準備を定める。
* [データの保存場所](data-locations.md) - 学習者データ（`{app_data_dir}`）とアプリ同梱データ（`{app_resources}`）のディレクトリ構成と、ファイルの書き込みの規則を定める。
* [リポジトリ構成](repository.md) - リポジトリ直下のディレクトリ（`documents/`、`content/`、`prompts/`、`runner/`、`tools/`、`src/`、`src-tauri/`）の役割を定める。
* [Tauri の権限](tauri-permissions.md) - フロントエンドに許可する Tauri の権限（capabilities）の範囲と、外部サイトをアプリ内で表示しないことを定める。
* [アーキテクチャの受け入れ条件](acceptance.md) - アーキテクチャの仕様を満たしたと判断する条件を定める。
