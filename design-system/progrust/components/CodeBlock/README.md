# CodeBlock

問題文・解説の中に置く短いコード。

- `panel` の上では `raised` の塗り（`.pg-code`）、`bg` の上では `panel` の塗り（`.pg-code--on-bg`）。角 `radius-inset`、書体 `code-sm`。
- 色分けは `.tk-k`（キーワード）・`.tk-s`（文字列）・`.tk-m`（マクロ）・`.tk-c`（コメント）・`.tk-t`（型）・`.tk-f`（関数）。
- 注意: ライトの `panel` の上では `code-comment` が 4.42:1 になる。コメントを含むコードは `raised` か `bg` の上に置く。
