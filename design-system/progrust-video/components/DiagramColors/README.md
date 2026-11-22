図（`figure.diagram > svg`）の色クラス `fill-*` / `stroke-*` の見た目で、塗り・線・文字が章カラーとライト／ダークでどうなるかを示す。

## 使い方

| クラス | 塗り（`fill-*`） | 線（`stroke-*`） | `text` に付けたとき |
| --- | --- | --- | --- |
| `primary` | 章カラー。中の文字は `fill-on` | 章カラー | 章カラーの文字（`video-primary-ink`） |
| `accent` | 章の淡い面（章外では第4章の淡い面） | 章カラーの濃い段（`video-accent-ink`） | 章カラーの文字 |
| `ok` | 成功の緑（`video-ok`）＋ ✓ を描く | 成功の緑 | 緑の文字（`video-ok-ink`） |
| `ng` | 赤（`video-ng`）＋ × を描く | 赤 | 赤の文字（`video-ng-ink`） |
| `muted` | 灰の面（`video-muted-fill`） | 灰（`video-muted-stroke`） | 灰の文字（`video-muted`） |
| （なし） | 塗りなし | `video-diagram-line`（= `ink-3`） | `video-diagram-text`（= `ink`） |

- 1 つの図形に `fill-*` と `stroke-*` を 1 つずつ付けられる: `<rect className="fill-accent stroke-accent" …/>`。
- `text` に付けた `fill-*` は、地（`video-bg`・`fill-muted`・`fill-accent`）の上で読める「文字の色」になる。`fill-primary` / `fill-ng` の濃い塗りの上の文字は `fill-on` にする。
- 線を描かない図形には `stroke="none"` を書く（既定で線が付く）。塗らない図形には何も書かない（既定は塗りなし）。
- 色クラスは図形と `text` に直接付ける（`g` には付けない。まとめて動かす・不透明度を変えるために `g` を使うのはよい）。
- 色クラスの意味は仕様の `content-format/video/colors.md`、色の直接指定の禁止は仕様の `content-format/video/prohibited-code.md` に従う。`fill`・`stroke` の属性に色を書かない（`none` は書いてよい）。
- 色が変わる動き（所有者の印が付く・外れる）は、色を補間せず、色クラスの違う図形を重ねて、不透明度で入れ替える。

## 追加のクラス

| クラス | 意味 |
| --- | --- |
| `fill-on` | 濃い塗り（`fill-primary`・`fill-ok`・`fill-ng`）の上の文字。ライトは白、ダークは濃い墨 |
| `stroke-on` | 濃い塗りの上に描く ✓ / × などの線 |
| `fill-surface` | 地の色（`video-bg`）で塗る。灰の領域の上に箱を置くとき |
| `text` の `code` | 変数名・型名・値などの短いラベルを等幅（Red Hat Mono・太さ 600）で書く。図の中に書いてよい文字は仕様の `content-guidelines/video.md`「図」 |

理由: 色クラスの 5 つだけでは、濃い塗りの上の文字の色（ライトで白、ダークで濃い色）を指定できない。灰の領域の上に白い箱を置くこともできない。
