図を主役にする配置（`data-layout="diagram"`）で、見出し・図 1 つ・callout か本文を置く。目に見えない仕組みを図で示すとき（仕様の `content-guidelines/video.md`「図」）に使う。

## 書き方

```tsx
<div className="video-stage" data-layout="diagram">
  <h1>代入すると、所有権が移る</h1>
  <figure className="diagram">
    <svg viewBox="0 0 1000 360" width={1000} height={360} role="img" aria-label="s1 は使えなくなり、s2 がヒープ上の hello を指す">
      …（DiagramShapes・DiagramPatterns の描き方に従う）
    </svg>
  </figure>
  <p>String を代入しても、ヒープ上のデータは複製されません。</p>
</div>
```

- 図は内容の領域の中で縦横とも中央に置かれる。
- 図の大きさ: 幅は内容の幅まで。高さは、見出しの下の高さから、添える callout か本文の分（約 130）を引いた値まで（README「1 画面に載せる量」）。1 倍なら幅 1136・高さ 400（添えないなら 520）、1.16 倍なら幅 1023・高さ 300（添えないなら 430）。
- `svg` の `width`・`height` は `viewBox` の幅・高さと同じ値にする（1 単位 = 基準の 1px。線の太さと文字の大きさがそろう）。
- 図の配色と形は Diagram の各部品（DiagramColors・DiagramShapes・DiagramPatterns）に従う。図の動かし方は DiagramPatterns。
- 見本の図（ムーブ）は「スタックとヒープ」の定型。領域 → 箱 → 矢印 → セルの順に描く。
- 図の中に書いてよい文字と、図とコードの分け方は仕様の `content-guidelines/video.md`「図」 に従う。見本の図で使う注記は「使えない」「所有者」。
- 図とコードを 1 つの画面に並べるときは、左右に並べる（仕様の `content-guidelines/video.md`「図」）。この配置ではなく、位置を決めて組む（README「ステージ」、見本は Camera）か、`two-column` を使う。
