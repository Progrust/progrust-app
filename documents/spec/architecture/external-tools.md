---
type: Specification
title: "外部ツール（cargo・claude）への依存"
description: "cargo と claude の検出方法と必要条件、見つからない場合の動作と導入手順の URL、rustc のバージョンが足りない場合の案内、許可クレートの初回準備を定める。"
---

この文書は、Rust の演習に使う `cargo` と、AI 機能に使う `claude` への依存を定める。C の演習に使う `clang` への依存は [C のツール（clang）への依存](c-toolchain.md) に定める。

## 検出

アプリ起動時と、設定画面・初回起動画面の「再検出」ボタンを押したときに検出する。

| ツール | 探索順 | 必要条件 |
|---|---|---|
| `cargo` / `rustc` | 設定で指定したパス → `PATH` → `~/.cargo/bin` | `rustc --version` が 1.100.0 以上 |
| `claude` | 設定で指定したパス → `PATH` → `~/.local/bin` → `~/.claude/local` → npm のグローバルインストール先（`/opt/homebrew/bin` → `/usr/local/bin`） | `claude --version` が成功すること |

- `PATH` に見つかった場合も見つからなかった場合も、表の既定のインストール先を探索する（macOS で Finder から起動した場合を含む）。
- `cargo` / `rustc` は、探索順の場所ごとに、`cargo` と、同じディレクトリの `rustc` を組で調べる。
  - 設定で指定するパス（[設定画面と settings.json](../screens/settings-screen.md)「settings.json」の `tool_paths.cargo`）は、`cargo` の実行ファイルの絶対パスとする。`rustc` は、そのファイルと同じディレクトリの `rustc` を使う。
  - `PATH` の場所は、`PATH` の各ディレクトリを順に調べる。相対パスのディレクトリは調べない。
  - `cargo` と `rustc` がどちらも実行できるファイルで、`rustc --version` が成功してバージョンを取り出せた最初の場所を使う。それ以降の場所は調べない。`rustc` のバージョンが必要条件に満たないときも、その場所を使う（「バージョンが足りない場合」）。
  - 次の場所は使えないとして飛ばし、次の場所を調べる。設定で指定したパスが無いか、相対パスである。`cargo` か `rustc` が無いか、実行できない。`rustc --version` が失敗するか、10 秒で終わらないか、出力からバージョンを取り出せない。
  - 使える場所が無いときは、見つからないとする（「見つからない場合」）。
- `rustc --version` の出力の 1 行目は `rustc 1.100.0 (…)` の形で、2 つ目の語をバージョンとする。必要条件と比べるときは、メジャー・マイナー・パッチの数を数として比べ、`-nightly` などの後ろの部分は比べない。
- `claude` は、探索順の場所ごとに、そのディレクトリの `claude` を調べる。
  - 設定で指定するパス（`tool_paths.claude`）は、`claude` の実行ファイルの絶対パスとする。
  - `PATH` の場所は、`cargo` と同じく、`PATH` の各ディレクトリを順に調べ、相対パスのディレクトリは調べない。
  - npm のグローバルインストール先は、`npm` を起動して調べず、Homebrew の Node.js（Apple シリコン）の `/opt/homebrew/bin` と、公式のインストーラーの Node.js の `/usr/local/bin` を調べる。nvm などで入れた Node.js の場所は調べない（設定でパスを指定する）。
  - `claude` が実行できるファイルで、`claude --version` が 10 秒で終了コード 0 で終わり、出力の 1 行目の最初の語が空でない最初の場所を使う。その語をバージョンとする（例: `2.1.329 (Claude Code)` の `2.1.329`）。
  - 次の場所は使えないとして飛ばし、次の場所を調べる。設定で指定したパスが無いか、相対パスである。`claude` が無いか、実行できない。`claude --version` が失敗するか、10 秒で終わらないか、出力が空である。
  - 使える場所が無いときは、見つからないとする（「見つからない場合」）。
- `claude --version` を起動するときに渡す環境変数は、[claude -p の呼び出しの共通ルール](../claude-integration/invocation.md)「環境変数」に従う。
- 検出結果（パス・バージョン）はメモリに保持し、`get_environment_status` で返す。

## 見つからない場合

| ツール | 動作 |
|---|---|
| `cargo` | Rust の演習の実行・テスト・提出のボタンを無効にし、演習画面とトップページに rustup の導入手順を案内する。動画の視聴と C の演習の実行・提出などは利用できる |
| `claude` | AI 機能（質疑応答・問題生成）を無効にし、Claude Code の導入手順を案内する。学習記録・分析のジョブは登録だけ行い、`claude` が使えるようになってから実行する（[claude が使えない場合の動作](../claude-integration/unavailable.md)） |

- 導入手順は、次の URL を OS の既定ブラウザで開いて案内する（[Tauri の権限](tauri-permissions.md)）。アプリが開く外部の URL は、この 2 つだけである。

| ツール | 導入手順の URL |
|---|---|
| `cargo`（rustup） | `https://rustup.rs` |
| `claude` | `https://code.claude.com/docs/ja/setup` |

## バージョンが足りない場合

- `rustc` は見つかったが、バージョンが必要条件（「検出」）に満たない場合は、演習画面とトップページに、今のバージョンと必要なバージョンを示し、`rustup update stable` で更新する手順を案内する。
- 実行・テスト・提出のボタンは無効にしない。

アプリから Rust や Claude Code を自動でインストールすることはしない。

## 初回準備

初回起動時、および許可クレートの一覧が変わったとき（アプリの更新時）に、許可クレートのダウンロードとビルドをバックグラウンドで行う。詳細は [許可クレートと作業領域](../exercise-runner/crates-and-workspace.md) を参照。
