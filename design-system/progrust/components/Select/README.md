# Select

候補から 1 つを選ぶセレクト（モデルの選択、エディタの文字サイズ、動画の再生速度）。（新規の案。D2 のモックアップにない部品を、D2 の面・角丸・文字の決まりで作った）

- 見た目は TextField と同じ。右端に Lucide の `chevron-down`（`muted`）を重ねる。
- OS の `<select>` を使い、開いた一覧は OS に任せる。
- `.pg-select--pill`（`.pg-select-wrap--pill` の中に置く）: ボタンと同じ行に並べる丸い形。高さ 36px・`field` の塗り・太さ 700 で、幅は中身に合わせる。動画のプレーヤーの再生速度（VideoControls）に使う。ラベルを出さないので `aria-label` を付ける。
