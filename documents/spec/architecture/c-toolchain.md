---
type: Specification
title: "C のツール（clang）への依存"
description: "C の演習のビルドに使う `clang` の検出方法（設定の `tool_paths.clang` → `PATH` → `/usr/bin`）と必要条件とバージョンの取り出し方、`clang` が見つからない場合の動作と Command Line Tools の導入の案内（`xcode-select --install` を画面に表示し、URL を開かないこと）、`clang` を同梱せず自動で導入しないことを定める。"
---

C の演習（C のレッスンの演習）のビルドには `clang` を使う（[C の演習の作業領域とビルド](../exercise-runner/c-project-assembly.md)）。提出のメモリリークの検査に使うライブラリは、アプリが用意する（[C のコードの提出（submit）](../exercise-runner/c-submit.md)「メモリリークの検査」）。Rust の演習に使う `cargo` と、AI 機能に使う `claude` は [外部ツール（cargo・claude）への依存](external-tools.md) に定める。

## 検出

`clang` は、`cargo`・`claude` と同じとき（アプリ起動時と、設定画面・初回起動画面の「再検出」ボタンを押したとき。[外部ツール（cargo・claude）への依存](external-tools.md)「検出」）に検出する。

| ツール | 探索順 | 必要条件 |
|---|---|---|
| `clang` | 設定で指定したパス → `PATH` → `/usr/bin` | `clang --version` が成功すること |

- `PATH` に見つかった場合も見つからなかった場合も、`/usr/bin` を探索する（macOS で Finder から起動した場合を含む）。
- 探索順の場所ごとに、そのディレクトリの `clang` を調べる。
  - 設定で指定するパス（[設定画面と settings.json](../screens/settings-screen.md)「settings.json」の `tool_paths.clang`）は、`clang` の実行ファイルの絶対パスとする。
  - `PATH` の場所は、`PATH` の各ディレクトリを順に調べる。相対パスのディレクトリは調べない。
  - `clang` が実行できるファイルで、`clang --version` が 10 秒で終了コード 0 で終わり、出力の 1 行目が空でない最初の場所を使う。それ以降の場所は調べない。
  - 次の場所は使えないとして飛ばし、次の場所を調べる。設定で指定したパスが無いか、相対パスである。`clang` が無いか、実行できない。`clang --version` が失敗するか、10 秒で終わらないか、出力が空である。
  - 使える場所が無いときは、見つからないとする（「見つからない場合」）。
- バージョンは、`clang --version` の出力の 1 行目で、`version` の次の語とする（例: `Apple clang version 21.0.0 (clang-2100.4.1.1)` の `21.0.0`）。取り出せないときも、その場所を使い、バージョンは無いものとする。バージョンは表示に使い、必要条件には使わない。
- Command Line Tools が入っていない Mac で、`clang` の有無を確かめる方法は決まっていない（[アーキテクチャの未決事項](open-questions.md)）。
- 検出結果（パス・バージョン）はメモリに保持し、`get_environment_status` で返す（[環境のコマンドとイベント](commands/environment.md)「環境の状態」）。

## 見つからない場合

- `clang` が見つからないときは、C の演習の実行・提出のボタンを無効にし、演習画面とトップページに、Command Line Tools の導入を案内する。
- 案内では、ターミナルで実行するコマンド `xcode-select --install` を画面に表示する。導入手順の URL は開かない。アプリが開く外部の URL は、[外部ツール（cargo・claude）への依存](external-tools.md)「見つからない場合」の表のものだけである。
- Rust の演習の実行・テスト・提出と、動画の視聴、質疑応答は、`clang` の有無に左右されない。
- C の演習の実行・提出は、`cargo` の有無と許可クレートの初回準備の状態に左右されない（[C の演習の作業領域とビルド](../exercise-runner/c-project-assembly.md)「作業領域」）。
- 初回起動画面・設定画面での検出結果の表示は、[初回起動画面](../screens/first-launch.md) と [設定画面と settings.json](../screens/settings-screen.md) に従う。

## 同梱と導入

- アプリに `clang` を同梱しない。
- アプリから Command Line Tools を自動でインストールすることはしない。`xcode-select --install` も、アプリからは実行しない。
