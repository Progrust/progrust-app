---
type: Specification
title: "C の演習の作業領域とビルド"
description: "C の演習の実行・提出のたびに作る `c-project/` の置き場所と中身（`main.c`、実行ファイル `main`・`main-checked`、検査の報告とメモリリークの検査の結果のファイルを置く `reports/`）、受け付けるファイルパス（`main.c` だけ）、検査なしと検査つきの `clang` のビルドのコマンドとオプション、ビルドの出力（`clang` の診断。色を付ける）の表示と送るとき、ビルド時間の制限、許可クレートの初回準備と `cargo` の有無に左右されないことを定める。"
---

C の演習（C のレッスンの演習。[演習のファイル形式](../content-format/exercise.md)「C の演習」）は、`clang`（[C のツール（clang）への依存](../architecture/c-toolchain.md)）でビルドする。実行は [C のコードの実行（run）](c-run.md)、提出は [C のコードの提出（submit）](c-submit.md) に従う。

## 作業領域

C の演習の作業領域は `{app_data_dir}/workspace/c-project/` とする（[許可クレートと作業領域](crates-and-workspace.md)「作業領域」）。Rust の演習の `project/` とは別のディレクトリである。

実行・提出のたびに `c-project/` を空にしてから、`main.c` を置く。ビルドとテストケースの実行で、次のものができる。

| ファイル | 内容 |
|---|---|
| `main.c` | エディタの `main.c` |
| `main` | 検査なしのビルドの実行ファイル（「ビルド」）。実行と、提出のメモリリークの検査で使う |
| `main-checked` | 検査つきのビルドの実行ファイル。提出のときだけ作る |
| `reports/` | メモリの検査の報告のファイル（[C のコードの提出（submit）](c-submit.md)「手順」の 2・3）と、メモリリークの検査の結果のファイル（同じく「手順」の 4）。提出のときだけ作る |

- `clang` がビルドで作るほかのファイル（デバッグ情報の `main.dSYM/` など）も、`c-project/` に置かれる。
- エディタから受け取るファイルは、`main.c` の 1 つだけとする。`main.c` のほかのパス（`..` を含むパスや絶対パスを含む）があるときと、`main.c` が無いときは拒否する。
- C の演習は許可クレートを使わない。許可クレートの初回準備（[許可クレートと作業領域](crates-and-workspace.md)「初回準備」）とは関係せず、初回準備の状態にかかわらず、`cargo` が見つからなくても、ビルド・実行・提出できる。結果の状態 `crates_unavailable` は、C の演習では起きない。

## ビルド

`c-project/` を作業ディレクトリとし、検出した `clang` を、新しいプロセスグループで起動してビルドする。ビルドは次の 2 種類である。

| ビルド | 使うとき | コマンド |
|---|---|---|
| 検査なし | 実行（[C のコードの実行（run）](c-run.md)）と、提出のメモリリークの検査 | `clang -std=c17 -g -O0 -Wall -Wextra -fcolor-diagnostics -o main main.c` |
| 検査つき | 提出のテストケースの実行（[C のコードの提出（submit）](c-submit.md)） | `clang -std=c17 -g -O0 -Wall -Wextra -fcolor-diagnostics -fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -o main-checked main.c` |

- `clang` が終了コード 0 で終わったら成功、それ以外は失敗とする。失敗したときの結果の状態は `compile_error` である。
- ビルド時間の制限（[実行・判定の制限値](limits.md)「C の演習」）は、`clang` の起動 1 回ごとに当てる。中断するときは、`clang` のプロセスグループ全体に SIGKILL を送る。

## ビルドの出力の表示

- ビルドの出力は、`clang` の標準出力と標準エラー出力（コンパイラの診断。`error:`・`warning:`・`note:` の行など）とする。受け取った順に、そのまま表示する。
- 診断には、`-fcolor-diagnostics` で ANSI エスケープの色が付く。コンソールは、Rust の演習のコンパイラの診断と同じく、ANSI の色を反映して表示する（[演習画面](../screens/exercise-screen.md)）。
- ビルドに成功したときも、ビルドの出力（コンパイラの警告など）を表示する。
- ビルドの出力を送るときと捨てるときは、[コードの実行（run）](run.md)「ビルドの出力の表示」の、ビルドの出力を送る決まりに従う（`crates_unavailable` の決まりは当てはまらない）。
- 提出の 2 つのビルドのうち、どちらの出力を表示するかは [C のコードの提出（submit）](c-submit.md)「手順」に従う。
