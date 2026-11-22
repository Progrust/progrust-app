文字の色クラス（`c-primary`・`c-accent`・`c-ok`・`c-ng`・`c-muted`）の見た目と使い分け。文中では `strong` に付ける。

## 使い分け

| クラス | 見た目 | 使う場面 |
| --- | --- | --- |
| `c-primary` | 章カラーの文字（`video-primary-ink`）・太さ 700 | その画面の主役。同時に 1〜2 か所 |
| `c-accent` | 章カラーの文字（`video-accent-ink`）・太さ 500 | 主役に関係する補助の語。多用しない |
| `c-ok` | 成功の緑（`video-ok-ink`）・太さ 700・先頭に ✓ | 正しい、コンパイルが通る、できる |
| `c-ng` | 赤（`video-ng-ink`）・太さ 700・先頭に × | 誤り、エラーになる、できない |
| `c-muted` | 灰（`video-muted`）・太さ 400 | 背景的な情報、無効になったもの |

```tsx
<p>s2 に代入すると、<strong className="c-primary">所有権が移ります</strong>。</p>
<td><strong className="c-ng">できない</strong></td>
```

- 色クラスの意味は仕様の `content-format/video/colors.md` に従う。動画のコードで色を直接書くと、lint でエラーになる（仕様の `content-format/video/prohibited-code.md`）。
- 太さは色クラスが決める（`strong` に付けても `c-accent` は 500、`c-muted` は 400）。
- `c-ok` と `c-ng` の ✓ / × は CSS が付ける。作者は「✓」「×」を書かない。
- `c-ok`（緑）と `c-ng`（赤）は章によらない固定の色。`c-primary`・`c-accent` だけが章カラーに従う。
- 第2章・第12章（緑の章）では `c-primary` と `c-ok` が似た色になる。✓ の形と文言で区別する。
- 章外（`data-chapter` なし）では `c-primary`・`c-accent` が第4章の色（マゼンタ）になる。
- 図では `fill-*` / `stroke-*` を使う（DiagramColors）。
