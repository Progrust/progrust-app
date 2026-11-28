---
type: Decision
title: "ADR 0272"
description: "C の演習の提出で、誤りの行がある報告のファイルだけを memory_error にし、メモリの検査の報告の UndefinedBehaviorSanitizer の文と AddressSanitizer の SUMMARY の種類から kind を、最初のスタックの main.c のフレームから location を決め、report を SUMMARY の行までにし、kind に stack_overflow を足し、ASAN_OPTIONS に allocator_may_return_null=1 と external_symbolizer_path=/usr/bin/atos を足す"
---

- 日付: 2026-11-28
- 関連仕様: [C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「検査の報告の読み取り」、[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「手順」、[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「判定結果」、[C の演習の作業領域とビルド](../spec/exercise-runner/c-project-assembly.md)「作業領域」
- 置き換え: なし

## 背景

C の演習の提出では、メモリの検査が誤りを報告したテストケースを `memory_error` にし、誤りの種類（`kind`）と行（`location`）と報告の先頭（`report`）を表示すると決めた（[ADR 0266](0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)）。報告からそれらを取り出す方法は、コード実行・判定の未決事項に残っていた。

Claude は、Issue #293 で技術検証をした（[C の演習のメモリの検査の報告から誤りの種類と行を取り出せるかの技術検証](../verification/issue-293-parse-sanitizer-reports.md)）。決め手になったことは次のとおりである。

- 配列の名前に直接添字を付けた範囲外アクセスは、AddressSanitizer ではなく UndefinedBehaviorSanitizer が `index 5 out of bounds for type 'int[5]'` と報告した。NULL の読み書きも、学習者のコードの中なら UndefinedBehaviorSanitizer が報告した。
- AddressSanitizer の `SUMMARY:` の行は、標準ライブラリの関数（`fgets`・`sprintf` など）の中で起きた誤りでは、`main.c` の行を指さなかった。報告の最初のスタックで `main.c` が出る最初のフレームは、試したすべての報告で、誤りの起きた行だった。
- 行番号は、AddressSanitizer が `PATH` から探した `atos` で付けていた。`PATH` に `/usr/bin` が無いと、行番号が出ず、報告に作業領域の絶対パスが出た。`external_symbolizer_path=/usr/bin/atos` を足すと、`PATH` によらずに行番号が出た。`main-checked.dSYM/` を消しても、行番号が出なくなった。
- 大きすぎる確保は、検査なしの実行では `malloc` が `NULL` を返すが、検査つきの実行では AddressSanitizer がその場で止めた。`NULL` を確かめて正しく終わるプログラムも、誤りの報告になった。`allocator_may_return_null=1` を足すと、検査つきでも `NULL` が返ったが、AddressSanitizer は `WARNING: AddressSanitizer failed to allocate …` の 1 行を報告のファイルに書いた。誤りの報告のファイルには、どれも `ERROR: AddressSanitizer` を含む行か `runtime error:` を含む行があった。
- 止まらない再帰は `stack-overflow` と報告され、今の `kind` の一覧のどれにも当たらなかった。
- 報告の後半（シャドウメモリの表と凡例、レジスタの値）は、学習者が読めるものではない。先頭から `SUMMARY:` の行までなら、最大で 22 行ほどで、誤りの起きた行・解放した行・確保した行が入る。

Claude は、画面に出るものが変わる 2 点をユーザーに尋ねた。

- `kind` の一覧に当たらない報告に、種類を足すか。案は、「スタックのあふれ」だけを足す（Claude が勧めた）、「スタックのあふれ」と「ブロックを抜けた変数の使用」を足す、足さない、の 3 つである。
- `report` をどこまでにするか。案は、`SUMMARY:` の行まで（Claude が勧めた）、最初のスタックまで、全文、の 3 つである。

ユーザーは、どちらも Claude が勧めた案を選んだ。

## 決定

- `kind` に `stack_overflow`（スタックのあふれ）を足す。ブロックを抜けた変数の使用（`stack-use-after-scope`）などは、`other` のままとする（[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「判定結果」）。
  - 理由: ユーザーが選んだ。Claude が勧めた理由は、c02-04 で「再帰とスタックの深さ」を教えるので、止まらない再帰を「そのほかのメモリの誤り」ではなく、名前で示せることである。ブロックを抜けた変数は、C の章で教えない。
- `report` は、報告の先頭から `SUMMARY:` の行までとし、区切りの行とレジスタの値の表を除く（[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「検査の報告の読み取り」）。
  - 理由: ユーザーが選んだ。Claude が勧めた理由は、誤りの起きた行に加えて、解放した行と確保した行が分かり、学習者の読めない表を含まないことである。
- 次の細部は Claude が決めた。
  - `kind` は、UndefinedBehaviorSanitizer の報告（`runtime error:` の行）があればその文から、無ければ AddressSanitizer の `SUMMARY:` の行の種類の語から決める（同上）。理由: 上の「背景」のとおり、配列の範囲外と NULL の読み書きは、どちらの検査が報告するかが、コードの形で変わる。
  - `location` は、UndefinedBehaviorSanitizer では `runtime error:` の行の先頭の行番号から、AddressSanitizer では最初のスタックの、`main.c` が出る最初のフレームから取る（同上）。理由: 上の「背景」のとおり、`SUMMARY:` の行は `main.c` を指さないことがある。
  - `SEGV`・`BUS` の報告で、`main.c` のフレームより前に `free` のフレームがあるものは、`invalid_free` にする（同上）。理由: 文字列リテラル・グローバル変数・初期化していないポインタを `free` したときは、AddressSanitizer の `free` の中で `BUS` になり、`bad-free` と報告されなかった。
  - 報告のファイルが複数あるときは、起動したプロセスのプロセス ID の名前のファイルを使う（同上）。理由: 学習者のプログラムが `fork` したとき、親と子のそれぞれが報告した。
  - `ASAN_OPTIONS` に `allocator_may_return_null=1` と `external_symbolizer_path=/usr/bin/atos` を足す（[C のコードの提出（submit）](../spec/exercise-runner/c-submit.md)「手順」）。理由: 上の「背景」のとおり。前者で、確保に失敗したときの動きが検査の有無で変わらず、c02-05 で教える `NULL` の確認が検査つきでも意味を持つ。後者で、アプリが受け取った `PATH` によらずに行番号が付き、報告に学習者のホームディレクトリの名前が入らない。
  - `memory_error` にするのは、報告のファイルに `ERROR: AddressSanitizer` を含む行か `runtime error:` を含む行があるときに限る。ファイルができただけでは `memory_error` にしない（同上）。理由: 上の「背景」のとおり、`allocator_may_return_null=1` では、確保に失敗した正しいコードでも、警告だけのファイルができる。
  - `main-checked.dSYM/` は、提出のテストケースをすべて実行し終えるまで消さない（[C の演習の作業領域とビルド](../spec/exercise-runner/c-project-assembly.md)「作業領域」）。理由: 上の「背景」のとおり、消すと行番号が出ない。

## 検討した他の案

- 案: 「スタックのあふれ」と「ブロックを抜けた変数の使用」の 2 つを `kind` に足す
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、ブロックを抜けた変数は C の章で教えないことである
- 案: `kind` を足さない
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、c02-04 で教える止まらない再帰が「そのほかのメモリの誤り」と表示されることである
- 案: `report` を最初のスタックまでにする
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、解放した行と確保した行が出ないことである
- 案: `report` を報告の全文にする
  - 採用しなかった理由: ユーザーが選ばなかった。Claude が示した難点は、学習者が読めないシャドウメモリの表と凡例が、約 30 行入ることである
- 案: `location` を AddressSanitizer の `SUMMARY:` の行から取る
  - 採用しなかった理由: 上の「背景」のとおり、標準ライブラリの関数の中で起きた誤りで、`main.c` の行を指さない

## 影響

- `kind` の決め方は、AddressSanitizer・UndefinedBehaviorSanitizer の報告の書式に頼る。Command Line Tools の更新で書式が変わったときの対応は、コード実行・判定の未決事項に残る。技術検証では、誤りの形ごとの `main.c` と、期待する `kind`・`location` を、実際の `clang` でビルドして確かめる自動テストを置けば、書式の変化に気づけることが分かった。このテストは作者の Mac でだけ動く。
- `report` は英語のままで、アドレスなどの数字も残る。
