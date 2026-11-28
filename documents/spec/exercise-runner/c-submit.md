---
type: Specification
title: "C のコードの提出（submit）"
description: "C の演習の提出の手順（検査なしと検査つきの 2 つのビルドとメモリリークの検査のライブラリのビルド、検査つきの実行ファイルでのテストケースの実行と `ASAN_OPTIONS`、メモリリークの検査のライブラリを `DYLD_INSERT_LIBRARIES` で差し込んだ検査なしの実行ファイルでのメモリリークの検査）、テストケースの結果の決め方（`memory_error` を含む）、メモリリークの検査のライブラリが数えるものと結果のファイル、検査の報告から `kind`・`location`・`report` を取り出す規則、メモリリークの検査のライブラリの作り方（バックエンドに埋め込んだソースを提出のたびに `clang` でビルドすること）と数えるものと結果のファイル、メモリリークの検査の起動がふつうに終わらなかったときの結果、判定結果の `cases[].status` の `memory_error` と `detail` の形（`kind` の一覧と日本語の名前、`location`、`report`）、画面に表示する内容を定める。"
---

C の演習（[C の演習の作業領域とビルド](c-project-assembly.md)）の、演習画面の「提出」の手順を定める。判定はテストケース（`cases.json`。[演習のファイル形式](../content-format/exercise.md)「C の演習」）で行い、メモリの検査をする。

次のことは、Rust の演習と同じく [コードの提出（submit）](submit.md) に従う。

- 提出中のコンソールの表示と、提出を学習者が途中で止められないこと（冒頭）
- 出力の正規化（「stdout 方式」の「正規化」）
- 判定結果の項目（「判定結果」）。C の演習で違うところは、下の「判定結果」に書く
- 途中で止めた提出（「途中で止めた提出」）

制限値は [実行・判定の制限値](limits.md)「C の演習」に従う。

## 手順

1. 検査なしのビルドと、検査つきのビルド（[C の演習の作業領域とビルド](c-project-assembly.md)「ビルド」）を、この順に行う。
   - 検査なしのビルドに失敗したら、そのビルドの出力を表示して、`compile_error` で終了する。検査つきのビルドは行わない。
   - 検査なしのビルドに成功したら、そのビルドの出力を表示する。
   - 検査つきのビルドに失敗したら、そのビルドの出力を表示して、`compile_error` で終了する。成功したときは、そのビルドの出力を表示しない。
   - 2 つのビルドに成功したら、メモリリークの検査のライブラリをビルドする（下の「メモリリークの検査」）。ビルドの出力は表示しない。失敗したとき（ビルド時間の制限を超えたときを含む）は、`compile_error` にせず、提出を、アプリの側の失敗（[実行のコマンドとイベント](../architecture/commands/run.md)「イベント」の `run_finished` の `error`）として終える。
2. 最初のテストケースを実行する前に、`c-project/reports/` を作る。テストケースごとに、`cases.json` の順に、`c-project/main-checked` を、`c-project/` を作業ディレクトリとして、新しいプロセスグループで起動する。
   - 環境変数 `ASAN_OPTIONS` に `detect_stack_use_after_return=1:allocator_may_return_null=1:external_symbolizer_path=/usr/bin/atos:log_path=reports/case-{N}` を設定する。`log_path` は、作業ディレクトリ（`c-project/`）からの相対パスで書く。`{N}` は、テストケースの `cases.json` での位置（最初を 1 とする）である。
   - `allocator_may_return_null=1` で、確保できない大きさの `malloc` などは、検査なしの実行と同じく `NULL` を返す。`external_symbolizer_path=/usr/bin/atos` で、検査の報告の行番号を、`PATH` によらずに付ける。
   - `stdin` を書き込んで標準入力を閉じ、標準出力を受け取る。標準エラー出力は、[コードの提出（submit）](submit.md)「stdout 方式」の 2 と同じく、`detail` に使う先頭の部分だけを受け取る。
3. 次のとおりテストケースの結果を決める。上の行から順に当てはめる。

   | 結果 | 条件 |
   |---|---|
   | `timeout` | 1 テストケースの実行時間の制限（[実行・判定の制限値](limits.md)）で止めた |
   | `fail` | 標準出力が 1 テストケースの出力量の上限（[実行・判定の制限値](limits.md)）を超えた（出力過多） |
   | `memory_error` | メモリの検査が誤りを報告した。`reports/` に、名前が `case-{N}.` で始まるファイル（`log_path` に指定したパスの後ろに `.` と文字が続く名前のファイル）ができ、そのどれかに、`ERROR: AddressSanitizer` を含む行か `runtime error:` を含む行があるときとする。確保に失敗したときの警告（`WARNING: AddressSanitizer failed to allocate`）だけのファイルは、誤りの報告として扱わない |
   | `runtime_error` | 終了コードが 0 以外で終わった、またはシグナルで終わった。標準出力が期待値と一致していても `runtime_error` にする |
   | `fail` | 終了コード 0 で終わり、正規化した標準出力が、正規化した `expected_stdout` と一致しない |
   | （4 へ進む） | 終了コード 0 で終わり、正規化した標準出力が、正規化した `expected_stdout` と一致した |

4. 3 で 4 へ進んだテストケースだけ、メモリリークを検査する（下の「メモリリークの検査」）。`c-project/main`（検査なしの実行ファイル）を、`c-project/` を作業ディレクトリとして、新しいプロセスグループで起動し、同じ `stdin` を書き込んで標準入力を閉じる。
   - 環境変数 `DYLD_INSERT_LIBRARIES` に、メモリリークの検査のライブラリ（`c-project/leakcheck/libprogrust_leakcheck.dylib`）の絶対パスを設定する。あわせて、環境変数 `PROGRUST_LEAK_REPORT` に、結果のファイルのパス `reports/leak-{N}` を設定する。結果のファイルのパスは、作業ディレクトリ（`c-project/`）からの相対パスで書く。`{N}` は 2 と同じである。
   - 標準エラー出力は、2 と同じく `detail` に使う先頭の部分だけを受け取る。この起動での学習者のプログラムの出力は、判定に使わない。
   - 次のとおりテストケースの結果を決める。上の行から順に当てはめる。

     | 結果 | 条件 |
     |---|---|
     | `timeout` | 1 テストケースの実行時間の制限（[実行・判定の制限値](limits.md)「C の演習」）で止めた。止め方は [コードの実行（run）](run.md)「停止」と同じである |
     | `runtime_error` | 終了コードが 0 以外で終わった、またはシグナルで終わった |
     | `runtime_error` | 終了コード 0 で終わったが、結果のファイルができなかった、読めなかった、形に合わなかった、または `overflow` だった（メモリリークを検査できなかった） |
     | `memory_error`（`kind` は `leak`） | 終了コード 0 で終わり、結果のファイルの、解放されなかった領域の件数が 1 以上 |
     | `pass` | 終了コード 0 で終わり、結果のファイルの、解放されなかった領域の件数が 0 |

   - ここでの `runtime_error` の `detail` は、下の「判定結果」の `runtime_error` と同じものとする。メモリリークを検査できなかったときは、標準エラー出力の先頭の後に、次の 1 行を続ける。結果のファイルが `overflow` のときは `メモリリークを検査できませんでした（同時に確保している領域が多すぎます）。`、それ以外は `メモリリークを検査できませんでした（プログラムが、main から戻るか exit を呼ぶ以外の方法で終わりました）。` とする。
5. 1 件失敗しても、残りのテストケースはすべて実行する。

- 判定の進み具合（`submit_progress`。[実行のコマンドとイベント](../architecture/commands/run.md)「イベント」）の `build` は 1 のビルド（メモリリークの検査のライブラリのビルドを含む）の間、`judge` は 2〜5 の間とする。テストケースの結果が決まるのは、4 へ進んだテストケースでは 4 が終わったときである。
- 2 の検査の報告（`reports/` の、名前が `case-{N}.` で始まるファイル）から `kind`・`location`・`report` を取り出す方法は、下の「検査の報告の読み取り」に従う。

## メモリリークの検査

手順の 4 のメモリリークの検査には、アプリが用意するメモリリークの検査のライブラリ（確保と解放を数える動的ライブラリ）を使う。

- ライブラリのソース（C の 1 ファイル）は、バックエンドのクレートに埋め込む。提出のたびに、手順の 1 で `c-project/leakcheck/leakcheck.c` に書き出し、検出した `clang` で `c-project/leakcheck/libprogrust_leakcheck.dylib` にビルドする（[C の演習の作業領域とビルド](c-project-assembly.md)「ビルド」）。教材チェックツールの判定（[教材チェックツール](../content-guidelines/check-tool.md)）も、同じ手順でビルドする。
- ライブラリは、手順の 4 の起動でだけ、環境変数 `DYLD_INSERT_LIBRARIES` で、検査なしの実行ファイルに差し込む。検査つきの実行ファイルと、実行（[C のコードの実行（run）](c-run.md)）には差し込まない。
- ライブラリは、`malloc`・`calloc`・`realloc`・`free`・`strdup` を、dyld の interpose で置き換える。
- ライブラリは、学習者の実行ファイルのコード（`main.c` からビルドしたコード）が `malloc`・`calloc`・`realloc`・`strdup` で確保した領域を記録し、`free` と `realloc` で解放された領域を記録から消す。解放は、どこから呼ばれたものでも記録から消す。
  - 学習者の実行ファイルのコードから呼ばれたかどうかは、呼び出し元のアドレスが、学習者の実行ファイル（読み込まれたイメージのうち、ファイルの種類が実行ファイルのもの）の `__TEXT` セグメントの中にあるかで決める。
  - 標準ライブラリが内部で確保した領域（`printf` のバッファ、`getline` が確保したり広げたりした領域など）は数えない。
  - 同時に記録できる領域は 524,288 個までとする。超えたときは、それ以降を記録せず、結果のファイルに `overflow` と書く。
- ライブラリは、読み込まれたときに、環境変数 `DYLD_INSERT_LIBRARIES` と `PROGRUST_LEAK_REPORT` を消す。学習者のプログラムが起動する子プロセスには、ライブラリを差し込まない。
- プログラムが `exit` で終わるとき（`main` から戻ったときを含む）に、記録に残っている領域（プログラムが終わるまでに解放されなかった領域）の件数と合計のバイト数を、`PROGRUST_LEAK_REPORT` のパスの結果のファイルに書き出す。結果のファイルには、件数と合計のバイト数を、この順に、10 進の整数で 1 行ずつ書く。上限を超えたときは、`overflow` の 1 行だけを書く。
  - 書き出すのは、ライブラリが読み込まれたプロセスだけとし、そのプロセスが `fork` で作った子プロセスは書き出さない。
  - `_exit`・`abort`・シグナルで終わったときは、結果のファイルを書かない。

## 検査の報告の読み取り

手順の 3 で `memory_error` にしたテストケースの `kind`・`location`・`report`（下の「判定結果」）は、検査の報告から次のとおり決める。

1. 使う報告のファイルは、`reports/` の、名前が `case-{N}.` で始まり、手順の 3 の誤りの行があるファイルのうち、名前の `case-{N}.` の後ろが、手順の 2 で起動したプロセスのプロセス ID のものとする。無ければ、名前の順で最初のものとする（学習者のプログラムが `fork` で子プロセスを作ったときは、ファイルが複数できる）。
2. `{ファイル}:{行}:{桁}: runtime error: {文}` の形の行があれば、UndefinedBehaviorSanitizer の報告とし、最初のその行で決める。
   - `kind`: `{文}` が `index ` で始まり `out of bounds for type` を含めば `out_of_bounds`、`null pointer` を含めば `invalid_access`、それ以外は `undefined_behavior` とする。
   - `location`: `{ファイル}` が `main.c` なら `main.c:{行}`、そうでなければ `null` とする。
3. 2 に当たらず、`SUMMARY: AddressSanitizer: {種類}` の形の行があれば、AddressSanitizer の報告とし、その行の `{種類}`（空白までの語）で決める。
   - `kind`: 下の表のとおりとする。表に無い `{種類}` は `other` とする。
   - `location`: 報告の最初のスタック（最初に `#0` で始まるフレームの行から、`#` で始まるフレームの行が続く範囲）のうち、`main.c:{行}` が出る最初のフレームの `main.c:{行}` とする。そのようなフレームが無ければ `null` とする。

   | `{種類}` | `kind` |
   |---|---|
   | `heap-use-after-free` | `use_after_free` |
   | `double-free` | `double_free` |
   | `heap-buffer-overflow`、`stack-buffer-overflow`、`stack-buffer-underflow`、`global-buffer-overflow`、`dynamic-stack-buffer-overflow` | `out_of_bounds` |
   | `stack-use-after-return` | `use_after_return` |
   | `bad-free` | `invalid_free` |
   | `SEGV`、`BUS` | `invalid_access`。ただし、最初のスタックの、`main.c:{行}` が出る最初のフレームより前に、関数の名前が `free` で始まるフレームがあれば `invalid_free` |
   | `stack-overflow` | `stack_overflow` |

4. 2 にも 3 にも当たらなければ、`kind` は `other`、`location` は `null` とする。
5. `report` は、報告の先頭から `SUMMARY:` の行までのうち、`=` だけの行と、レジスタの値の表（`==` とプロセス ID と `==Register values:` の行と、それに続く、レジスタの値の行）を除いたものとする。`SUMMARY:` の行が無いときは、報告の全体から同じ行を除いたものとする。

- 行番号は、AddressSanitizer が `atos` と `main-checked.dSYM/` から付ける（[C の演習の作業領域とビルド](c-project-assembly.md)「作業領域」）。

## 判定結果

判定結果の項目（`status`・`passed`・`total`・`cases[]`）は [コードの提出（submit）](submit.md)「判定結果」と同じとし、次のところが違う。

- `cases[].status` には、`pass`・`fail`・`timeout`・`runtime_error` のほかに、`memory_error`（メモリの検査が誤りを報告した）がある。`memory_error` は C の演習だけで使う。
- `status` は `passed`・`wrong`・`compile_error`・`stopped` のどれかで、`crates_unavailable` にはならない。`memory_error` のテストケースが 1 件でもあれば、`wrong` になる。

`detail` の形:

| テストケースの結果 | `detail` |
|---|---|
| `pass` | `null` |
| `fail` | [コードの提出（submit）](submit.md)「判定結果」の stdout 方式の `fail` と同じオブジェクト `{ "input": string, "expected": string, "actual": string }` |
| `timeout` | 標準エラー出力の先頭（文字列） |
| `runtime_error` | 標準エラー出力の先頭（文字列）。シグナルで終わったときは、その後に [C のコードの実行（run）](c-run.md)「シグナルで終わったとき」の 1 行を続ける。手順の 4 でメモリリークを検査できなかったときは、その後に手順の 4 の 1 行を続ける |
| `memory_error` | オブジェクト `{ "kind": string, "location": string \| null, "report": string }`（下の表） |

```json
{ "kind": "use_after_free", "location": "main.c:10", "report": "（検査の報告の先頭）" }
```

| 項目 | 内容 |
|---|---|
| `kind` | 誤りの種類（下の「誤りの種類」の値） |
| `location` | 誤りが起きた `main.c` の行。`main.c:{行番号}` の形（例: `main.c:10`）。報告から取り出せないときは `null` とする。`leak` では、常に `null` とする |
| `report` | 検査の報告の先頭。`leak` では、解放されなかった領域の件数と合計のバイト数を伝える文 `解放されていない領域が {件数} 件あります（合計 {バイト数} バイト）。` とする |

**誤りの種類**

| `kind` | 日本語の名前 | 意味 |
|---|---|---|
| `use_after_free` | 解放後の使用 | `free` した領域を読み書きした |
| `double_free` | 二重解放 | 同じ領域を 2 回 `free` した |
| `out_of_bounds` | 範囲外アクセス | 配列や確保した領域の範囲の外を読み書きした |
| `use_after_return` | 戻った関数のローカル変数の使用 | 呼び出しから戻った関数のローカル変数を、ポインタを通して読み書きした |
| `invalid_free` | 確保していない領域の解放 | `malloc` などで確保していない領域を `free` した |
| `invalid_access` | NULL や無効なアドレスへのアクセス | NULL ポインタや、どの領域も指していないアドレスを読み書きした |
| `undefined_behavior` | 未定義動作 | 上のどれでもない未定義動作（符号付き整数のあふれなど） |
| `stack_overflow` | スタックのあふれ | 関数の呼び出しが深くなりすぎて、スタックを使い切った（止まらない再帰など） |
| `leak` | メモリリーク | 確保した領域を `free` しないまま、プログラムが終わった |
| `other` | そのほかのメモリの誤り | メモリの検査の報告のうち、上のどれにも分けられないもの |

## 画面に表示する内容

- 通過数と失敗したテストケースの名前と、最初に失敗したテストケースの `detail` を表示することは、[コードの提出（submit）](submit.md)「判定結果」と同じである。`memory_error` のテストケースも、失敗したテストケースに含める。
- `memory_error` の `detail` は、`kind` の日本語の名前と `location` を見出しにし（例: 「解放後の使用（main.c:10）」。`location` が `null` のときは名前だけ）、その下に `report` を表示する。
- 表示する場所と、詳細の開閉は [演習画面](../screens/exercise-screen.md) に従う。

## 試行ログ

試行ログへの記録は [実行・提出の試行ログへの記録](attempt-log.md) に従う。
