# ProblemPanel

演習画面の左の列に置く問題文の面と、問題文・解説の本文（`.pg-prose`）。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- 面は `.pg-side`（`panel`）。上に小さな見出し（`.pg-side-head`、`ink-3`）とテストの件数の Tag、中は `.pg-side-scroll` でスクロールする。
- 本文は `.pg-prose`。見出しは `title-sm`、小見出しは `subheading`、本文と箇条書きは `body` の `ink-2`。文中のコードは `raised` の小さな面に `code-sm`。`bg` の上に置くとき（正解のモーダル）は `.pg-prose--on-bg` を足し、文中のコードを `panel` にする。
- 入力と出力の例は `.pg-io`（`dt` に「入力」「出力」）。
- 提出の結果（JudgeResult）は、この面の上（問題文の前）に置く。
