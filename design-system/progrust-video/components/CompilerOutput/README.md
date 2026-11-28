ターミナルの出力（台本の `code_blocks` の `language` が `text`。出力は `pre > code.language-text`。コンパイラ・cargo の出力、プログラムの実行結果、標準入力とのやりとり）で、白い地を 2px の線（`video-rule`）で囲み、18px の等幅で表示する。行番号は付けない。コードブロック（灰の地・行番号あり）と見分けられる。

## 書き方

台本の `code_blocks` の 1 件（`code` は、行を `\n` で区切った 1 つの文字列。ここでは読みやすいように行で分けて示す）:

```text
error[E0382]: borrow of moved value: `s1`
 --> src/main.rs:4:16
  |
2 |     let s1 = String::from("hi");
  |         -- move occurs because `s1` has type `String`, which does not implement the `Copy` trait
3 |     let s2 = s1;
  |              -- value moved here
4 |     println!("{s1}");
  |                ^^ value borrowed here after move
```

- `code_blocks` の書き方は仕様の `content-format/video/script-json.md`「`code_blocks` の 1 件」・`content-format/video/code-display.md` に従う。
- どこで実際の rustc の出力を示すか（そのエラーをレッスンで初めて説明するところ）、実際にコンパイルして得た出力を使うこと、載せる行を書き換えないこと、エラーの箇所を波線で示すこと、エラーの説明のしかたは仕様の `content-guidelines/video.md`「エラー」 に従う。出力はエラーコードも含めてそのまま書く。出力の中では、マーカーと波線を使わない（仕様の `content-format/video/code-display.md`「`CodeBlock` の指定」）。
- 同じ画面のコードの行番号を出力とそろえること（`start_line`）は仕様の `content-guidelines/video.md`「コード」 に従う。例: 出力の `4 |`・`src/main.rs:4:16` は、同じ画面のコードの左に表示される 4 行目と同じ行を指す（preview の見本）。
- ターミナル以外のテキスト（入力ファイル、設定ファイルの中身など）は `text` にしない（仕様の `content-format/video/code-display.md`「言語（`language`）」）。`plaintext`（色付けなし・行番号あり）か、その言語の名前で書く（CodeLabel）。
- 同じ画面のコードでは、出力の `^^^` が指す箇所に波線、原因の箇所（`-- value moved here` など）にマーカーを引くと対応が分かりやすい。
- 出力を読み上げるあいだは、読んでいる行と、その行が指すコードの行だけを残して、ほかの行を淡くしてよい（CodeMotion）。
- 1 行に入る字数は（ブロックの幅 − 56）÷ 10.8（全幅で 100 字）。長い行は折り返さずに、ブロックの右端で切れる。出力の先頭の `warning` や `Compiling` の行など、説明に使わない行は省く。載せる行が入りきらないときは、コードブロックを包む要素にクラス `fade-right` を付けて、行の右端をぼかして消す（`--fade-right`。README「共通部品が出力する HTML の形」）。
- 出力は 10 行まで。コードと出力を両方載せるなら合わせて 14 行まで（README「1 画面に載せる量」）。
- 文字は 18px なので、読み上げるところでは 1.12 倍以上に寄せる（README「文字の最小サイズ」）。
- 出力の行は、上から順に 2 フレームに 1 行ずつ出してよい（README「動きの種類と速さ」）。
- 実行結果（標準出力）も同じ書き方。

## rustc の出力の自動の色付け

共通部品が rustc の出力を行ごとに解析して、rustc の端末での色に合わせて次の `span` で囲む:

| クラス | 対象 | 色 |
| --- | --- | --- |
| `out-error` | `error[E0382]` | `video-ng-ink`・600 |
| `out-warning` | `warning` と warning の `^^^` | `video-warn`・600 |
| `out-note` | エラーの見出しの本文、`note:` | `video-heading`・600 |
| `out-loc` | `-->`、行番号と `|`、二次的な下線 `--` とその説明 | `video-out-loc`（= `link`）・600 |
| `out-caret` | error の `^^^` とその説明 | `video-ng-ink`・600 |
| `out-help` | `help:` | `video-out-help`（= `ansi-cyan`）・600 |

作者は色を書かない（出力をそのまま書くだけ）。理由: エラーの行と原因の行が一目で分かり、アプリのコンソール（ANSI の色）と見た目がそろう。

## clang の診断とメモリの検査の報告の自動の色付け

C のレッスンの出力（仕様の `content-format/video/code-display.md`「言語（`language`）」）も、共通部品が行ごとに解析して、上と同じクラスで囲む。新しいクラスは作らない。動画では、rustc の出力と同じ意味の区分にそろえる（アプリのコンソールでは clang の warning は `ansi-magenta` だが、動画では `out-warning` にする）。

clang の診断（`main.c:5:10: error: …` の形の行と、続くコード片の行・`^` の行・修正の案の行）:

```text
main.c:12:25: error: expected ';' after expression
   12 |     printf("%d\n", f(1))
      |                         ^
      |                         ;
```

| クラス | 対象 |
| --- | --- |
| `out-loc` | 行の先頭の位置（`main.c:12:25:`）と、コード片の行・`^` の行の行番号と `|` |
| `out-error` | `error:` |
| `out-warning` | `warning:` と、warning の後の `^`・`~` |
| `out-note` | `note:` と、error・warning・note の後の本文 |
| `out-caret` | error と note の後の `^`・`~` |
| `out-help` | 修正の案（`^` の行の次に clang が出す、差し替える文字の行の文字） |

- コード片の行（`   12 |` の後ろ）の字句には色を付けない。端末の clang はコード片の字句にも色を付けるが、動画では、どの行を読めばよいかを `^` と位置で示す。

メモリの検査の報告（AddressSanitizer・UndefinedBehaviorSanitizer）:

| クラス | 対象 |
| --- | --- |
| `out-error` | `ERROR: AddressSanitizer: …` の行の `ERROR: AddressSanitizer: {種類}`、`SUMMARY:` の行、`runtime error:` |
| `out-loc` | `runtime error:` の行の先頭の位置（`main.c:7:12:`）と、スタックの行（`#0 0x… in main main.c:10`）の `main.c:{行}` |
| `out-note` | 「どこで解放したか」「どこで確保したか」の見出しの行（`freed by thread T0 here:`・`previously allocated by thread T0 here:`） |

- スタックの行のアドレスと、標準ライブラリの関数の行は色を付けない。読ませたい `main.c` の行だけが目立つ。
