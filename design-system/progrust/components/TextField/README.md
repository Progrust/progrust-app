# TextField

1 行の入力欄と、複数行の入力欄（`.pg-textarea`）。（新規の案。D2 のモックアップにない部品を、D2 の面・角丸・文字の決まりで作った）（複数行はチャットの入力欄を元にした）

- `raised` の塗りに `shadow-control` の内側の線。1 行は高さ 38px・角 `radius-inset`、複数行は角 `radius-panel`。
- ラベルは欄の上に `.pg-field` で置き、補足は `.pg-field-help`、誤りは `.pg-field-error`（`danger-ink` ＋アイコン）と `aria-invalid="true"`。
- パスなど英数字の値は `.pg-input--mono`。
- 注意: `control-line` は 3:1 に届かない。`panel` の上では面の差で欄が分かるが、`bg` の上に置くときは見え方を確かめる。
