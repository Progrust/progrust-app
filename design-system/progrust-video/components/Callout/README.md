強調ボックス（クラス `callout`）の 5 種類（`info`・`tip`・`warn`・`ng`・`ok`）で、左にアイコン（Lucide）、地の色で種類を示す。色は 5 種類とも章によらない固定の色（意味と色が一般に結びついているため）。

## 使い分け

| 種類 | 地 | アイコン | 使う場面 |
| --- | --- | --- | --- |
| `info` | 情報の青（`video-info-fill`） | `info`（`video-info-icon`） | 補足、背景の知識、他の言語との違い |
| `tip` | ヒントの紫（`video-tip-fill`） | `lightbulb`（`video-tip-icon`） | こうすると良い、別の書き方 |
| `warn` | 注意の橙（`video-warn-soft`） | `triangle-alert`（`video-warn`） | 気をつけること、性能・落とし穴 |
| `ng` | 誤りの赤（`video-ng-soft`） | `circle-x`（`video-ng-ink`） | 誤り、エラーになる |
| `ok` | 成功の緑（`video-ok-soft`） | `circle-check`（`video-ok`） | 正しい、コンパイルが通る |

```tsx
<div className="callout ng">ムーブしたあとの変数を使うと、ムーブ済みの値を使ったエラー（E0382）になります。</div>
```

- 中は 1〜2 文の文章（22px）。`code`・`strong`・`br` を使ってよい。リストやコードブロックを入れない。
- 文体は仕様の `content-guidelines/video.md`「画面の文章」、`ng` でエラーを書くときの説明のしかたは仕様の `content-guidelines/video.md`「エラー」 に従う。
- 幅は文の長さに合わせて縮む（最大は親の幅）。1 行に収まる長さが読みやすい。図やコードの下に置くときは、`max-width` をその図やコードの幅にする。2 行になるときの折り返しは CSS がそろえる（`text-wrap: balance`）。
- 同時に 1 つ（左右 2 列で比べるときは各列に 1 つ）。説明が進んだら、同じ位置で次の callout に入れ替える（README「1 画面に載せる量」「動きの種類と速さ」）。
- callout は、その内容を読み上げる位置で現れる（16 フレーム、下から 16px）。消えるときは 8 フレーム。
- アイコンは CSS が描く。「注意:」のような前置きを書かないことは仕様の `content-guidelines/video.md`「画面の文章」 に従う。
- `ok` と `ng` は色だけで区別しない（アイコンが ✓ と ×）。文の中でも何が正しい／誤りかを書く。
- 文字はどの地の上でも墨（`video-heading`、12:1 以上）。アイコンは地の上で 5.6:1 以上。
