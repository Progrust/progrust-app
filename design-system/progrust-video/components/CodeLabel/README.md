コードブロックに付けるファイル名や言語名のラベル（台本の `code_blocks` の `label`。出力は `pre[data-label]`）と、Rust 以外の言語のコード（`language` が `python` など）。

## 書き方

台本の `code_blocks`:

```json
[
  { "id": "py", "label": "Python", "language": "python", "code": "s1 = \"hi\"\ns2 = s1\nprint(s1)  # hi と表示される" },
  { "id": "rs", "label": "Rust（src/main.rs）", "language": "rust", "code": "let s1 = String::from(\"hi\");\nlet s2 = s1;" }
]
```

- `label` と `language` の書き方は仕様の `content-format/video/script-json.md`「`code_blocks` の 1 件」・`content-format/video/code-display.md`「言語（`language`）」 に従う。
- ラベルはコードブロックの上に 18px・太さ 700・`video-text-sub` で出る。テーマ CSS が `attr(data-label)` で描く。
- 使う場面: 他の言語との比較（仕様の `content-guidelines/target-learner.md`。並べ方は仕様の `content-guidelines/video.md`「コード」）、複数ファイル（`src/lib.rs` と `src/main.rs`）を並べるとき、ファイルの途中から載せるとき（仕様の `content-guidelines/video.md`「コード」）。
- Rust 以外の言語は色付けせず、コードブロックの見た目（灰の地・行番号あり）で表示する。
- ターミナル以外のテキスト（入力ファイル `input.txt`、`Cargo.toml` の中身など）も、`plaintext` かその言語の名前で書き、`label` にファイル名を書く。`text` はターミナルの出力専用（行番号なし。仕様の `content-format/video/code-display.md`「言語（`language`）」）。
- 理由: Rust 以外のコードを `text` で書くと出力と同じ見た目になり、比較の画面でどちらがどの言語か分かりにくい。
