---
type: Verification Report
title: "C の演習のメモリの検査の報告から誤りの種類と行を取り出せるかの技術検証"
description: "C の演習の提出で、AddressSanitizer・UndefinedBehaviorSanitizer の報告から誤りの種類（kind）と行（location）と報告の先頭（report）を取り出せるかを Issue #293 で検証した結果（kind ごとの報告の語、行を最初のスタックから取ること、行番号が atos と main-checked.dSYM に頼ること、PATH に /usr/bin が無いと行番号が消えて作業領域の絶対パスが出ること、報告が 1 つのファイルにまとまること、タイムアウトで止めたときの報告、実行時間とビルド時間、空白と日本語のパス、検査が報告しないもの、-Wall -Wextra の警告の出方、規則の案、スタックのあふれを kind に足して report を SUMMARY の行までにしたユーザーの判断）を記す。"
---

- 日付: 2026-11-28
- Issue: [#293](https://github.com/Progrust/progrust-app/issues/293)
- 確かめた OS: macOS 27.2.1（Darwin 27.2.0、Apple M1）
- 関連仕様: [C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)、[C の演習の作業領域とビルド](../spec/exercise-runner/c-project-assembly.md)、[実行・判定の制限値](../spec/exercise-runner/limits.md)、[コード実行・判定の未決事項](../spec/exercise-runner/open-questions.md)、[演習画面](../spec/screens/exercise-screen.md)、[c03 メモリの誤り](../spec/curriculum/chapters/c03-memory-errors.md)、[ADR 0266](../decisions/0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)

## 環境

- macOS 27.2.1（ビルド 26B131）、Apple M1
- `/usr/bin/clang`: `Apple clang version 21.0.0 (clang-2100.4.1.1)`、`Target: arm64-apple-darwin27.2.0`、`InstalledDir: /Library/Developer/CommandLineTools/usr/bin`
- Command Line Tools: `com.apple.pkg.CLTools_Executables` 27.2.0.0.1792721180。Xcode は入っていない。
- AddressSanitizer の実行時のライブラリ: `/Library/Developer/CommandLineTools/usr/lib/clang/21/lib/darwin/libclang_rt.asan_osx_dynamic.dylib`
- 報告の行番号を付ける外部のプログラム（シンボル化のプログラム）: `/usr/bin/atos`（macOS に付属する実行ファイル）

## 方法

- 作業領域を、空白と日本語を含むパス `…/Application Support/日本語 データ/workspace/c-project/` に作った。
- C の章の誤り（解放後の使用、二重解放、範囲外アクセス、戻った関数のローカル変数の使用、確保していない領域の解放、NULL や無効なアドレスへのアクセス、未定義動作）と、境界の例（標準ライブラリの関数の中で起きる誤り、`fork`、止まらない再帰、`abort`、`assert`、無限ループ、誤りのないプログラム など）を、79 本の `main.c` にした。
- 1 本ごとに `c-project/` を作り直し、仕様のとおりに次のことをした。
  - 仕様の 2 つのビルド（検査なし `clang -std=c17 -g -O0 -Wall -Wextra -fcolor-diagnostics -o main main.c` と、検査つき `… -fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -o main-checked main.c`）を、`c-project/` を作業ディレクトリとして行った。
  - `reports/` を作り、`./main-checked` を、`c-project/` を作業ディレクトリとし、新しいプロセスグループで起動した。`ASAN_OPTIONS` は `detect_stack_use_after_return=1:log_path=reports/case-{N}` とした。標準入力を書き込んで閉じ、5 秒で止まらなければプロセスグループに SIGKILL を送った。同じ実行ファイルを 2 回ずつ動かした。
  - 環境変数は、Finder から起動したアプリに近いもの（`PATH=/usr/bin:/bin:/usr/sbin:/sbin`、`HOME`、`TMPDIR`、`USER` だけ）にした。
- 条件を変えて、次の場合も動かした。環境変数を何も渡さない（`env -i`）。`PATH` を存在しないディレクトリにする。`main-checked.dSYM` を消してから動かす。`ASAN_OPTIONS` に `allocator_may_return_null=1` を足す。
- 報告から `kind`・`location`・`report` を決める規則の案（下の「規則の案」）を Python で試作し、できたすべての報告に当てた。
- 警告は、誤りの形ごとに 31 本の短いコードを、検査なしのビルドのオプションでビルドして調べた。

## 結果

### 報告のファイルと書式

- 誤りがあったプログラムでは、`reports/case-{N}.{プロセス ID}` という名前のファイルが 1 つできた。誤りの無いプログラム、`free(NULL)`、メモリリークだけのプログラム、`abort`、`assert` の失敗、終了コード 3 で終わるプログラム、初期化していない変数の読み取り、タイムアウトで止めたプログラムでは、ファイルはできなかった。
- 報告の書式は、AddressSanitizer（以下 ASan）と UndefinedBehaviorSanitizer（以下 UBSan）で違う。

ASan の報告（解放後の使用。`main.c` の 5 行目で確保、7 行目で解放、8 行目で読んだ）:

```
=================================================================
==42290==ERROR: AddressSanitizer: heap-use-after-free on address 0x602000000930 at pc 0x000100d94984 bp 0x00016f06b5a0 sp 0x00016f06b598
READ of size 4 at 0x602000000930 thread T0
    #0 0x000100d94980 in main main.c:8
    #1 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

0x602000000930 is located 0 bytes inside of 16-byte region [0x602000000930,0x602000000940)
freed by thread T0 here:
    #0 0x0001015f9368 in free+0x7c (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x41368)
    #1 0x000100d948f8 in main main.c:7
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

previously allocated by thread T0 here:
    #0 0x0001015f9274 in malloc+0x78 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x41274)
    #1 0x000100d94854 in main main.c:5
    #2 0x000193cc4d78 in start+0x19f8 (dyld:arm64e+0x31d78)

SUMMARY: AddressSanitizer: heap-use-after-free main.c:8 in main
Shadow bytes around the buggy address:
  （シャドウメモリの表と凡例が約 30 行続く）
==42290==ABORTING
```

UBSan の報告（符号付き整数のあふれ）:

```
main.c:6:11: runtime error: signed integer overflow: 2147483647 + 1 cannot be represented in type 'int'
SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior main.c:6:11
```

- UBSan の報告も、ASan と同じ `log_path` のファイルに書かれた。
- `-fno-sanitize-recover=all` で、どちらの検査も最初の誤りで止まるので、1 回の実行で報告は 1 つだった。ただし `fork` で子プロセスを作ったプログラムでは、親と子のそれぞれが報告し、ファイルが 2 つできた（`case-1.41867` と `case-1.41868`）。
- 誤りを報告したプログラムは、どれも SIGABRT で終わった。
- 報告をファイルに書いても、セグメンテーション違反などのシグナルを受けたときは、標準エラー出力に `AddressSanitizer:DEADLYSIGNAL` の 1 行が出た。
- 誤りの前に `printf` した標準出力（`line1`）は、受け取れた。

### 誤りの種類と報告の語

報告の中で種類を表す語は、ASan では `SUMMARY: AddressSanitizer: ` の次の語、UBSan では `runtime error: ` の後の文である。下の表の `kind` は、下の「規則の案」で決めたものである。

| 試した誤り | 報告した検査 | 報告の語 | `kind` |
|---|---|---|---|
| `free` した領域の読み書き（`main` の中、関数の中、`printf`・`puts`・`fputs`・`atoi` に渡す、連結リストを `free` しながらたどる、`realloc` で移った古い領域） | ASan | `heap-use-after-free` | `use_after_free` |
| 同じ領域の 2 回の `free`（ポインタの代入、構造体の浅いコピー） | ASan | `double-free` | `double_free` |
| ヒープの範囲外（読み、書き、`a[-1]`、構造体の配列、`strcpy`・`memcpy`・`memset`・`sprintf`、終端の `'\0'` の分を確保しなかった文字列の `strcpy`、終端の `'\0'` が無い文字列の `printf`、要素数を誤った `qsort`） | ASan | `heap-buffer-overflow` | `out_of_bounds` |
| スタックの配列の範囲外（ポインタで受け取った関数の中、`strcpy`・`strcat`・`scanf`・`fgets`） | ASan | `stack-buffer-overflow`、`stack-buffer-underflow` | `out_of_bounds` |
| グローバルの配列の範囲外（ポインタを通して） | ASan | `global-buffer-overflow` | `out_of_bounds` |
| 配列の名前に直接添字を付けた範囲外（ローカル、グローバル、可変長配列、負の添字） | UBSan | `index 5 out of bounds for type 'int[5]'` | `out_of_bounds` |
| 戻った関数のローカル変数の読み取り | ASan | `stack-use-after-return` | `use_after_return` |
| 領域の途中・スタックの変数・スタックの配列の `free` | ASan | `bad-free` | `invalid_free` |
| グローバル変数・文字列リテラル・初期化していないポインタの `free` | ASan | `BUS`（ASan の `free` の中で起きた） | `invalid_free` |
| NULL の読み書き（`*p`、`p->next`、`p[3]`、連結リストの末尾を越えてたどる） | UBSan | `load of null pointer …`、`store to null pointer …`、`member access within null pointer …`、`applying non-zero offset 12 to null pointer` | `invalid_access` |
| NULL を `strlen`・`memcpy` に渡す | ASan | `SEGV on unknown address 0x000000000000` | `invalid_access` |
| 決まった数のアドレス・初期化していないポインタの読み書き | ASan | `SEGV`、`BUS` | `invalid_access` |
| 符号付き整数のあふれ、0 での除算、大きすぎるシフト、境界のそろっていないアドレスの読み取り | UBSan | `signed integer overflow …`、`division by zero`、`shift exponent 40 is too large …`、`load of misaligned address …` | `undefined_behavior` |
| ブロックを抜けた変数の読み取り | ASan | `stack-use-after-scope` | `other` |
| 止まらない再帰 | ASan | `stack-overflow` | `other` |
| 重なる領域への `strcpy` | ASan | `strcpy-param-overlap` | `other` |
| 大きすぎる確保（`malloc(SIZE_MAX / 2)`） | ASan | `allocation-size-too-big` | `other` |

- 配列の範囲外は、添字を付ける式が配列の名前のとき（`a[i]` で `a` が `int a[5]`）は UBSan が、ポインタのとき（関数の引数、`malloc` の領域、`int *p = table`）は ASan が報告した。UBSan の文から種類を決めないと、配列の範囲外が `undefined_behavior` になる。
- NULL の読み書きは、学習者のコードの中なら UBSan が、標準ライブラリの関数の中なら ASan（`SEGV`）が報告した。検査なしではシグナルで終わる誤りで、報告なしで異常終了したもの（`runtime_error` になるもの）は無かった。報告が出なかった異常終了は、`abort`・`assert`（SIGABRT）と `__builtin_trap()`（SIGTRAP）だけだった。
- `puts(NULL)` は誤りにならず、`(null)` を出力して終了コード 0 で終わった。
- 大きすぎる確保は、検査なしでは `malloc` が `NULL` を返すが、検査つきでは ASan がその場で止めた。`NULL` を確かめて正しく終わるプログラム（`malloc(SIZE_MAX / 2)` と `calloc(SIZE_MAX / 2, 8)`）も、`allocation-size-too-big` の報告になった。
- `ASAN_OPTIONS` に `allocator_may_return_null=1` を足すと、検査つきでも `NULL` が返った。`NULL` を確かめないプログラムでは、続く `p[0] = 1` を UBSan が `store to null pointer` と報告した。ただし、`malloc` の失敗では、`NULL` を確かめて正しく終わるプログラムでも、ASan が報告のファイルに次の 1 行を書いた（`calloc` の失敗では書かなかった）。報告のファイルができたことだけで `memory_error` にすると、このプログラムが不正解になる。

```
==47680==WARNING: AddressSanitizer failed to allocate 0x7fffffffffffffff bytes
```

- 誤りを報告したファイルには、どれも `ERROR: AddressSanitizer` を含む行か `runtime error:` を含む行があった。

### 行（location）

- UBSan の報告では、`runtime error:` の行の先頭の `main.c:{行}:{桁}:` が、誤りが起きた行だった。
- ASan の報告では、`SUMMARY:` の行が、`main.c` の行を指さないことがあった。誤りが、ASan が置き換えていない標準ライブラリの関数の中で起きたときである。

```
（fgets(line, 100, stdin) で 4 バイトの配列があふれた。main.c の 5 行目）
    #0 0x0001047580a8 in memcpy+0x260 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x900a8)
    #1 0x000193dd03e4 in fgets+0x184 (libsystem_c.dylib:arm64e+0x1f3e4)
    #2 0x00010474e848 in fgets+0x30 (libclang_rt.asan_osx_dynamic.dylib:arm64e+0x86848)
    #3 0x000104060940 in main main.c:5
SUMMARY: AddressSanitizer: stack-buffer-overflow (libsystem_c.dylib:arm64e+0x1f3e4) in fgets+0x184
```

  `sprintf`・`strlen(NULL)`・`memcpy(dst, NULL, 4)` でも同じだった。
- 報告の最初のスタック（`ERROR:` の行の後に `#0` から続く行）の中で、`main.c:{行}` が出る最初のフレームを取ると、試したすべての ASan の報告で、誤りが起きた `main.c` の行になった。
  - 標準ライブラリの関数の中で起きた誤り（`strcpy`・`printf`・`memcpy`・`fgets`・`sprintf` など）では、その関数を呼んだ `main.c` の行になった。
  - 関数の中で起きた誤りでは、その関数の中の行になった（例: `read_first` の中の 15 行目。呼んだ `main` の 21 行目ではない）。
  - `qsort` に要素数を誤って渡したプログラムでは、比較の関数の中の、範囲外を読んだ行（5 行目）になった。誤りの原因の `qsort` の行（11 行目）ではない。
  - 止まらない再帰では、再帰する関数の最初の行（3 行目）になった。
- 報告には、誤りの起きた行のほかに、解放した行（`freed by thread T0 here:` のスタック）と、確保した行（`previously allocated by thread T0 here:` のスタック）がある。最初のスタックだけを使えば、誤りの起きた行が取れる。

### 行番号が出る条件

- 行番号は、ASan がプログラムの終了前に `atos` を起動して付けていた。`PATH` から `atos` を探す。
  - `PATH=/usr/bin:/bin:/usr/sbin:/sbin` では、行番号が出た。
  - 環境変数を何も渡さないときと、`PATH` に `/usr/bin` が無いときは、行番号が出ず、フレームに実行ファイルの絶対パスが出た。

```
    #0 0x0001023f89c8 in read_first+0xa4 (/private/tmp/…/Application Support/日本語 データ/workspace/c-project/main-checked:arm64+0x1000009c8)
SUMMARY: AddressSanitizer: heap-use-after-free (/private/tmp/…/c-project/main-checked:arm64+0x1000009c8) in read_first+0xa4
```

  - `ASAN_OPTIONS` に `external_symbolizer_path=/usr/bin/atos` を足すと、環境変数を何も渡さないときも、行番号が出て、絶対パスは出なかった。
- `main-checked.dSYM`（`-g` でビルドすると `clang` が作るデバッグ情報）を消すと、ASan の報告から行番号が消えた（`in main+0x150 (main-checked:arm64+0x100000980)`）。UBSan の報告の行番号は、ビルドのときに埋め込まれるので、消えなかった。
- 行番号が出たときの報告には、作業領域の絶対パスは出ず、ファイル名は `main.c` だけだった。

### 報告の先頭（report）

- 報告の全体は 2〜53 行だった。ASan の報告の後半（`Shadow bytes around the buggy address:` からの約 30 行）は、シャドウメモリの表と凡例で、学習者には読めない。`SEGV`・`BUS` の報告には、レジスタの値の表（`==PID==Register values:` から数行）が入る。
- 先頭から `SUMMARY:` の行までを取り、区切りの `====…` の行と、レジスタの値の表を除くと、2〜22 行、113〜1,353 バイトになった。誤りの種類、誤りの起きた行、解放した行と確保した行（解放後の使用・二重解放・範囲外アクセス）、対象の変数の名前（`'buf' (line 5) <== Memory access at offset 37 overflows this variable`）が、この中に入る。
- この範囲に入る英語の文（`READ of size 4 at … thread T0` など）とアドレスは、残る。

### 時間

- ビルドは、検査なし・検査つきのどちらも、1 回 0.06〜0.10 秒だった。554 行のプログラムでも 0.07 秒（検査なし）と 0.10 秒（検査つき）だった。
- 検査つきの実行ファイルの 1 回の起動は、次のとおりだった。ビルドの直後の 1 回目は、macOS が新しい実行ファイルを初めて起動するときの時間が加わる。

| 場合 | 1 回目 | 2 回目 |
|---|---|---|
| 誤りの無いプログラム（メモリリークだけのものを含む） | 0.20〜0.31 秒 | 0.014〜0.022 秒 |
| UBSan が報告した | 0.22〜0.31 秒 | 0.029〜0.040 秒 |
| ASan が報告した（`atos` の起動を含む） | 0.30〜0.42 秒 | 0.07〜0.15 秒 |
| `malloc` と `free` を 100 万回 | 0.37 秒 | 0.12 秒 |

- 1 テストケースの制限（5 秒）には、どれも十分に収まった。無限ループは 5 秒で止め、報告のファイルはできなかった。

### 空白と日本語を含むパス

- 作業領域が空白と日本語を含むパスでも、2 つのビルドと、相対パスの `log_path` への報告の書き出しができた。

### 検査が報告しないもの

- 初期化していないローカル変数・配列の要素・構造体のメンバー・ヒープの領域の読み取り。初期化していないヒープの領域は、検査つきの実行では毎回 `-1094795586`（`0xBEBEBEBE`。ASan が確保した領域を埋める値）を読み、検査なしの実行と違う値になった。
- メモリリーク（この環境の ASan では使えない。メモリリークは別のライブラリで検査する。[#294](https://github.com/Progrust/progrust-app/issues/294)）。
- `abort`・`assert` の失敗（報告は出ず、SIGABRT で終わる）。
- `puts(NULL)`（`(null)` を出力する）。

### -Wall -Wextra の警告

| 誤りの形 | 警告 |
|---|---|
| ローカル変数・ローカルの配列・ローカルの構造体のメンバーのアドレスを、`return &x;` の形でそのまま返す | 出る（`-Wreturn-stack-address`） |
| ローカル変数のアドレスを、ポインタの変数に入れてから返す。引数のポインタを通して呼び出し元に渡す | 出ない |
| 初期化していない変数を、そのまま使う（`return x;`、`sum += i`、`*p = 1`、`scanf("%d", n)` の `n`） | 出る（`-Wuninitialized`） |
| `if` で片方の場合だけ代入した変数を使う | 出る（`-Wsometimes-uninitialized`） |
| 初期化していない配列の要素・構造体のメンバー・ヒープの領域を読む。関数に渡して代入されなかった変数を使う | 出ない |
| 配列の範囲外を、定数の添字で書く（`a[3]` で `int a[3]`） | 出る（`-Warray-bounds`） |
| 配列の範囲外を、ループで書く（`i <= 3`） | 出ない |
| 大きさの分かる配列に、長すぎる文字列リテラルを `strcpy` する | 出る（`-Wfortify-source`） |
| スタックの変数を `free(&x)` する | 出る（`-Wfree-nonheap-object`） |
| スタックの変数のアドレスを入れたポインタを `free` する | 出ない |
| 解放後の使用、二重解放、メモリリーク、NULL の読み書き | 出ない |
| `memset(s, 0, sizeof(s))` で `s` がポインタ | 出る（`-Wsizeof-pointer-memaccess`） |
| 定数の式のあふれ・0 での除算 | 出る（`-Winteger-overflow`、`-Wdivision-by-zero`） |
| `printf`・`scanf` の書式と引数の型の食い違い | 出る（`-Wformat`） |
| 値を返す関数で、返さない経路がある | 出る（`-Wreturn-type`） |
| `int` と `strlen` の戻り値の比較 | 出る（`-Wsign-compare`。`-Wextra` で有効） |

### 規則の案

試作した規則は次のとおりである。できたすべての報告（79 本のうち報告が出た 64 件）で、`kind` は上の「誤りの種類と報告の語」の表のとおりに、`location` は誤りが起きた `main.c` の行になった。`main-checked.dSYM` を消したときは、ASan の報告の `location` が `null` になった。

最後に、決めた後の `ASAN_OPTIONS` の全文（`detect_stack_use_after_return=1:allocator_may_return_null=1:external_symbolizer_path=/usr/bin/atos:log_path=reports/case-{N}`）で、79 本に、`NULL` を確かめて正しく終わる 2 本を足した 81 本を動かし直した。誤りの行があるファイルは 64 件で、どれも SIGABRT で終わり、`kind`・`location` は、大きすぎる確保をして `NULL` を確かめないプログラムが `other`（`main.c:6`）から `invalid_access`（`main.c:7`。続く `p[0] = 1` の行）に変わったほかは、前と同じだった。誤りの行が無いファイルは、上の警告の 1 行だけのファイル（`malloc` の失敗を確かめて正しく終わるプログラム）の 1 件だった。

1. 使う報告のファイル: `reports/` の、名前が `case-{N}.` で始まり、`ERROR: AddressSanitizer` か `runtime error:` を含む行があるファイルのうち、名前の後ろが、起動したプロセスのプロセス ID のもの。無ければ、名前の順で最初のもの。そのようなファイルが無ければ、`memory_error` にしない。
2. `{ファイル}:{行}:{桁}: runtime error: {文}` の形の行があれば、UBSan の報告とする。
   - `kind`: `{文}` が `index … out of bounds for type` で始まれば `out_of_bounds`、`null pointer` を含めば `invalid_access`、それ以外は `undefined_behavior`。
   - `location`: `{ファイル}` が `main.c` なら `main.c:{行}`、そうでなければ `null`。
3. そうでなければ、`SUMMARY: AddressSanitizer: {語}` の行の `{語}` で決める。
   - `kind`: `heap-use-after-free` → `use_after_free`。`double-free` → `double_free`。`heap-buffer-overflow`・`stack-buffer-overflow`・`stack-buffer-underflow`・`global-buffer-overflow`・`dynamic-stack-buffer-overflow` → `out_of_bounds`。`stack-use-after-return` → `use_after_return`。`bad-free` → `invalid_free`。`SEGV`・`BUS` → `invalid_access`。ただし `SEGV`・`BUS` で、最初のスタックの、`main.c` のフレームより前に `free` のフレームがあれば `invalid_free`。それ以外の語は `other`。
   - `location`: 最初のスタック（`#0` から、`#` で始まる行が続く範囲）の中で、`main.c:{行}` が出る最初のフレームの `main.c:{行}`。無ければ `null`。
4. どちらの行も無ければ、`kind` は `other`、`location` は `null`。
5. `report`: 報告の先頭から `SUMMARY:` の行までのうち、`=` だけの行と、`==PID==Register values:` から続くレジスタの値の行を除いたもの。

規則の中心の部分（Python）:

```python
UBSAN = re.compile(r"^(?P<file>[^:\s]+):(?P<line>\d+):\d+: runtime error: (?P<msg>.*)$")
ASAN_SUMMARY = re.compile(r"^SUMMARY: AddressSanitizer: (?P<type>\S+)")
FRAME = re.compile(r"^\s+#\d+ 0x[0-9a-f]+ in (?P<func>\S+)(?: (?P<loc>\S+))?")
```

### 検証のあとの判断

検証のあと、ユーザーは、画面に出るものが変わる 2 点で Claude の推奨を選び、ほかを Claude の推奨に任せた。次のように決め、仕様に書いた（[ADR 0272](../decisions/0272-read-kind-and-location-from-sanitizer-reports.md)）。

- `kind` に `stack_overflow`（スタックのあふれ）を足す。`stack-use-after-scope` などは `other` のままとする（ユーザーの選択）。
- `memory_error` にするのは、報告のファイルに `ERROR: AddressSanitizer` か `runtime error:` を含む行があるときに限る（上の「規則の案」の 1）。
- `report` は、上の「規則の案」の 5 のとおり、`SUMMARY:` の行までとする（ユーザーの選択）。
- `kind`・`location` の決め方は、上の「規則の案」の 1〜4 のとおりとする。
- `ASAN_OPTIONS` に `allocator_may_return_null=1` と `external_symbolizer_path=/usr/bin/atos` を足す。

## 判定

基準を満たした。`kind` の一覧のそれぞれについて、報告から `kind` と `location` を決める規則（上の「規則の案」）を、仕様に書ける形で定められた。規則で決められない報告は `other` に、行を取り出せない報告は `location` を `null` にできる。

ただし、行番号が出るには、次の 2 つが要る。

- ASan が `atos` を見つけられること。アプリが渡す `PATH` に頼らないように、`ASAN_OPTIONS` に `external_symbolizer_path=/usr/bin/atos` を足す必要がある。
- `main-checked.dSYM` が、テストケースを実行する間、`c-project/` に残っていること（今の仕様では残る）。

## 残った事項

- 仕様の変更が要りそうな点と、Claude の推奨（検証のあとに決まった。上の「検証のあとの判断」）:
  - `ASAN_OPTIONS` に `external_symbolizer_path=/usr/bin/atos` を足す（推奨）。足さないと、`PATH` に `/usr/bin` が無いときに行番号が出ず、`report` に作業領域の絶対パス（ユーザーのホームディレクトリの名前を含む）が入る。
  - `ASAN_OPTIONS` に `allocator_may_return_null=1` を足す（推奨）。c02-05 で「確保に失敗したときの `NULL`」を教えるが、今の仕様では、検査つきの実行だけが、大きすぎる確保をその場で `other` として止める。足すと、検査なしの実行と同じく `NULL` が返り、`NULL` を確かめないコードは、続く読み書きで `invalid_access` になる。
  - 報告のファイルが複数できたとき（学習者のプログラムが `fork` したとき）は、起動したプロセスのプロセス ID の名前のファイルを使う（推奨）。
  - 今の `kind` の一覧に当たらない報告（`stack-use-after-scope`、`stack-overflow`、`strcpy-param-overlap`・`memcpy-param-overlap`、`allocation-size-too-big`）を、`other` のままにするか、種類を足すか（ユーザーの判断が要る。画面に出る名前が変わる）。c02-04 で「再帰とスタックの深さ」を教えるので、止まらない再帰が「そのほかのメモリの誤り」と表示される。
  - `report` の範囲を、上の「規則の案」の 5 にするか（ユーザーの判断が要る。学習者が読む文が変わる）。英語の文とアドレスが残る。
- Command Line Tools の更新で `clang` の版が変わったときに気づく方法: 上の表の誤りの形ごとに `main.c` と期待する `kind`・`location` を用意し、実際の `clang` でビルドして規則に当てる自動テストを置けば、報告の書式の変化に気づける。このテストは macOS の `clang` と `atos` に頼るので、作者の Mac でだけ動き、CI（ubuntu-latest）では動かせない。CI での扱いは、非機能要件の未決事項（[#294](https://github.com/Progrust/progrust-app/issues/294) の結果でも触れる）と合わせて決める。
- 確かめられなかった項目: `clang` の版が変わったときの報告の書式の変化（版が 1 つしか無い）。Intel の Mac と、ほかの版の macOS（対応環境の外。[ADR 0090](../decisions/0090-support-only-authors-mac-until-completion.md)）。
