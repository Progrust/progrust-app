# FormRow

入力の大きいフォームの 1 項目（生成設定画面）。左に項目名と説明、右に操作を置く。（画面デザインで追加した部品。D2 の面・角丸・文字の決まりで作った）

- `.pg-form` の中に `.pg-form-row` を並べる。左の列は 240px で、項目名は `subheading`、説明は `caption` の `muted`。右の列（`.pg-form-ctrl`）に操作を縦に積む。
- 行は `line-soft` の線で区切る。最後に `.pg-form-foot` を置き、右端に「キャンセル」と確定のボタンを並べる。
- 文字数などの補足は `.pg-count`（`caption`・`muted`、数字だけ `ink`）。
- 設定画面のように、1 行で済む操作の一覧には SettingsRow を使う。
