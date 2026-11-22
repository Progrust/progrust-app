文章と箇条書きが中心の配置で、`data-layout` を省いたときもこの配置になる。見出しの下に、上から積む。

## 書き方

```tsx
<div className="video-stage" data-layout="content">
  <h1>所有権には 3 つのルールがある</h1>
  <ol>
    <li>値には、<strong>所有者</strong>と呼ばれる変数が 1 つだけある</li>
    <li>所有者は、同時に 2 つ以上存在しない</li>
    <li>所有者がスコープを抜けると、値は<span className="mark">自動で破棄される</span></li>
  </ol>
  <div className="callout info">Java や Python のように、実行中に不要なメモリを探して回収する仕組みはありません。</div>
</div>
```

- 子要素は自由（`h2`・`p`・`ul`・`ol`・コードブロック・`table`・callout・`figure.diagram`・`img`）。上から順に `video-space-24` の間隔で積む。
- 量: 本文 1 つ（2 行まで）+ 箇条書き 4 項目、または箇条書き 5 項目。callout は 1 つまで（README「1 画面に載せる量」）。下に空きが残ってよい。
- `p` と `ul` の 1 行は最大 1000px（全角 41 字）。
- 箇条書きの項目は、読み上げに合わせて 1 つずつ出す。上から積む配置なので、あとから出す要素も先に置いておき、不透明度で出す（場所を空けておく）。
- 位置を決めて組む画面（コードと図を並べて変えていく画面）も、`data-layout` を付けないステージに作る（README「ステージ」、見本は Camera）。
- コードが主役なら `code`、図が主役なら `diagram`、比較なら `two-column` を使う。
