ファイルの途中から載せたコード（台本の `code_blocks` の `start_line`）で、行番号をファイルの行番号から始める。コンパイラの出力の行番号や、ナレーションと画面の文で指す行番号と同じ番号になる。

## 書き方

台本の `code_blocks` の 1 件:

```json
{
  "id": "main",
  "label": "src/main.rs",
  "language": "rust",
  "start_line": 118,
  "code": "fn main() {\n    let r;\n    {\n        let x = 5;\n        r = &x;\n    }\n    println!(\"{r}\");\n}"
}
```

- **書き方と制約は仕様書に従う**。`start_line` の書き方（値の範囲、Diff と併用するとき）、共通部品に指定する行番号の数え方、表示する最後の行番号の上限は仕様の `content-format/video/script-json.md`「`code_blocks` の 1 件」・`content-format/video/code-display.md`、途中から載せるときの書き方（`label` にファイル名を書くこと、どこから載せてよいか）と文での行の指し方は仕様の `content-guidelines/video.md`「コード」 を唯一の定義とする。
- 共通部品に指定する行番号も、表示される番号で書く。上の例に範囲の枠を重ねるなら、`r` を 119〜124、`x` を 121〜123 と指定する（CodeScope）。波線は 122 行目の `&x`、マーカーは 124 行目の `r` に引く。
- 見本の callout の文は「x の枠は 123 行目で終わるのに、r の枠は 124 行目まで続きます。…」のように、表示される番号で指している。
- 番号の欄には 3 桁まで入る（README「行番号」）。
- 見本は、同じエラーを前に出力とともに説明したあとの画面なので、出力を省いている。そのエラーを初めて説明するところでは出力（CompilerOutput）も示す（仕様の `content-guidelines/video.md`「エラー」）。

## Diff の例

```json
{
  "id": "fix",
  "label": "src/main.rs の変更",
  "language": "rust",
  "start_line": 12,
  "diff": true,
  "code": "     let s1 = String::from(\"hi\");\n-    let s2 = s1;\n+    let s2 = s1.clone();\n     println!(\"{s1} {s2}\");"
}
```

表示される番号は 12、（削除行は番号なし）、13、14。文では「13 行目のように `clone()` を足す」と言う。
