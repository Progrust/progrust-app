# SegmentedControl

少数の選択肢から 1 つを選ぶ部品（テーマ、難易度 1〜5）。（新規の案。D2 のモックアップにない部品を、D2 の面・角丸・文字の決まりで作った）

- `field` の地の中に選択肢を並べ、選択中は Tabs と同じく `raised` ＋ `shadow-tab`。
- `role="radiogroup"` と、各選択肢に `role="radio"` / `aria-checked` を付ける。
- 選択肢は 2〜5 個まで。それより多いときは Select を使う。
- 難易度のように選んだ値に説明があるときは、下に `.pg-field-help` で目安を書く。
