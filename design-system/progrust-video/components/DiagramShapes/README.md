図の形と寸法の決まりで、箱・セル・領域・矢印・破線・✓ と × の描き方を示す。値はすべて基準の px（`viewBox` の 1 単位 = 1px）。

## 決まり

- **svg**: `<svg viewBox="0 0 W H" width="W" height="H" role="img" aria-label="図の要点">`。W は内容の幅まで（1 倍で 1136。寄せたときは README「1 画面に載せる量」）。`role`・`aria-label` を付けることは仕様の `content-format/video/scenes.md` に従う。
- 下の例は SVG の属性名で書いている。TSX では `className`・`strokeWidth`・`strokeDasharray`・`textAnchor`・`dominantBaseline`・`fontSize`・`fontWeight` と書く。
- **文字**: `text` の既定は 20px・太さ 700（`video-fig-label`）。補足は `font-size="18" font-weight="500"`（`video-fig-note`）。18 未満にしない。18px の補足は、1.12 倍以上に寄せた画面で読ませる（README「文字の最小サイズ」）。変数名・型名・値は `class="code"`。中央に置くときは `text-anchor="middle" dominant-baseline="central"`。
- **箱**（変数・値）: 高さ 72〜80、角丸 `rx="12"`、線 2（既定）。名前は左から 28、補足は右から 20〜24 で右寄せ。
- **主役の箱**: `class="fill-primary" stroke="none"`、中の文字は `fill-on`。1 つの図に 1〜2 個。
- **補助の箱・セル**: `class="fill-accent stroke-accent"`、文字は `fill-accent`。
- **セル**（ヒープの 1 文字、配列の要素）: 56〜64 の正方形、角丸 `rx="8"`、間 8〜12。
- **無効になったもの**（ムーブ後の変数）: `class="fill-surface stroke-muted" stroke-dasharray="8 6"`、文字は `fill-muted`、「使えない」と書き添える。
- **領域**（スタック・ヒープ）: `class="fill-muted" stroke="none"`、角丸 `rx="20"`。名前は左上（左 28、上 40）に 18px で `fill-muted`。中に置く箱は `fill-surface`（灰の上に灰の線を引かない）。
- **矢印**: 線 3（`stroke-width="3"`）。矢じりは同じ色クラスの開いた山形（辺 10）を別の `path` で描く: `<path class="stroke-primary" d="M578 248 L 588 256 L 578 264" stroke-width="3"/>`。`marker` は使わない（色クラスが効かない）。矢印の先は相手の図形の手前 2〜12 で止める。
- **曲がる矢印**: 3 次ベジェ（`C`）で描き、両端は水平か垂直に入る。
- **破線**: `stroke-dasharray="8 6"`。無効になったもの、まだ存在しないもの、範囲の目安に使う。
- **✓ / ×**: 14 角・線 3。✓ は `d="M x y l7 7 l14 -14"`、× は `d="M x y l14 14 M x+14 y l-14 14"`。文字の左に置き、文言を必ず添える。
- 図の地は `video-bg`。図全体を枠で囲まない。影・グラデーションを使わない。
- **図の中に書いてよい文字と、図とコードの分け方は仕様書に従う**（`content-guidelines/video.md`「図」）。このシステムの図で使うラベルは、変数名・型名・値・`&s` や `'a` のような 1 語のラベルと、短い注記。変数が生きている範囲・スコープは、領域や破線などで図にせず、CodeScope で示す（同じ `content-guidelines/video.md`「図」）。
- **動かすとき**: 図形ごとに、不透明度・位置を現在のフレームから決める（README「動きの種類と速さ」）。線を根元から描くときは、`path` に `pathLength="1"` を付け、`stroke-dasharray`・`stroke-dashoffset` で伸ばす。矢じりは別の `path` なので、線が届くころに出せる。
