Rust のコードブロック（共通部品 `CodeBlock`。出力は `pre > code.language-rust`）で、灰の地（`video-code-bg`）に 20px の等幅で表示する。注目させたい字句はマーカー、エラーの箇所は波線、変更は Diff で示す。

## 書き方

- 画面に出すコードは、台本（`script.json`）の `code_blocks` に書き、共通部品 `CodeBlock` で表示する。同じコードを TSX に書き写さない。`code_blocks` の書き方と `CodeBlock` の指定のしかたは、仕様の `content-format/video/script-json.md`・`content-format/video/code-display.md` を唯一の定義とする。
- 行数と 1 行の字数は README「1 画面に載せる量」「行番号」による。幅いっぱいのときは、寄せの倍率に応じて 9〜12 行（callout か本文を添えるなら 5〜8 行）。図と左右に並べるときは、幅 480（1 行 32 字）で 8 行まで。
- 左に行番号が出る（共通部品が番号を渡し、CSS が描く）。番号の数え方は仕様の `content-format/video/code-display.md`「行番号と Diff」、文での行の指し方は仕様の `content-guidelines/video.md`「コード」 に従う。
- インデントは空白 4 つ。`code` の最後に改行を入れない（空行が表示される）。
- 注目させたい字句はマーカーで強調する（`marks`。CodeMark）。1 つのコードブロックで、同時に 1〜3 か所。行全体に色を付けて強調しないこと、読み上げている行に目を向けさせるときにほかの行を淡くしてよいことは仕様の `content-guidelines/video.md`「コード」 に従う（淡くする見た目は CodeMotion）。
- エラーの箇所は波線（`errors`。CodeError）。追加・削除した行は Diff（`diff`。CodeDiff）。ファイルの途中から載せるときは `start_line`（CodeExcerpt）。ファイル名・言語名は `label`（CodeLabel）。
- コードは、入力する・行を差し込むなどの動きで、ナレーションに合わせて変えていける（CodeMotion、README「動きの種類と速さ」）。
- コードの中のコメントは日本語で短く書いてよい（`// ここで text が破棄される`）。灰（`video-code-comment`）になる。
- 字句の色はアプリ側の Monaco Editor と同じ（README「コードの色」）。
- 演習の答えに直結する完成コードを載せない（仕様の `content-guidelines/video.md`「構成と長さ」）。

## 出力される HTML の形

共通部品は字句を `tk-*` の `span` で囲み、1 行ずつ `span.line` に入れる。行番号は、`span.line` の `data-ln` 属性で渡し、CSS が描く（要素は増やさない。README「共通部品が出力する HTML の形」）。この preview は `data-ln` を省き、予備の CSS のカウンタで描いている。マーカーと波線の字句は `span.mark` / `span.error` で囲み、その中の字句も `tk-*` で囲む（見本の preview はこの形）。作者はこの形を書かない。
