# CodeModal

前回の正解コードを読み取り専用で見せるモーダル。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `.pg-scrim` の中に `.pg-code-modal`（`raised`、`radius-section`、`shadow-pop`）を置く。見出しは `title-sm` と、正解した日時（`caption`・`muted`）。
- 中に EditorFrame を読み取り専用で置き、ファイルのタブで切り替える。足元は「閉じる」だけ。編集中のコードには影響しないことを、見出しの補足に書く。
