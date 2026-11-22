左右 2 列に分ける配置（`data-layout="two-column"`）で、見出しと `div.col` を 2 つ置く。比較（コピーとムーブ、他の言語と Rust、正しいコードと誤ったコード）に使う。

## 書き方

```tsx
<div className="video-stage" data-layout="two-column">
  <h1>i32 はコピーされ、String はムーブされる</h1>
  <div className="col">
    <h2>i32（コピー）</h2>
    <CodeBlock block="copy" />
    <div className="callout ok">a も b も使えます</div>
  </div>
  <div className="col">
    <h2>String（ムーブ）</h2>
    <CodeBlock block="move" />
    <div className="callout ng">s1 はムーブ済みなので使えません（E0382）</div>
  </div>
</div>
```

- 列の幅は 544px（`video-col-width`）、間は 48px（1 倍のとき）。列の中は上から `video-space-24` の間隔で積む。
- 寄せるときは、列を 500px・間を 40px に詰めて左に寄せ、1.12 倍で映す（`style` で `gridTemplateColumns: "500px 500px"`・`columnGap: 40`・`justifyContent: "start"`。README「寄せ方」）。
- 左右で同じ構成（小見出しかコードのラベル → コード → callout）にそろえると比べやすい。
- 左の列を先に、右の列を後に、読み上げに合わせて出す。列の中も、コード → callout の順に出す。
- 見本は、ムーブ済みの値を使ったエラーを前に出力とともに説明したあとの比較なので、出力を省いている。そのエラーを初めて説明するところでは出力も示す（仕様の `content-guidelines/video.md`「エラー」。列には収まらないので、コードと出力を上下に並べた画面にする）。
- 比較の順: 左が「よくある誤り」「前」、右が「正しい書き方」「後」。他の言語と比べるときの左右は仕様の `content-guidelines/video.md`「コード」 に従う。
- 量（各列）: 小見出しかラベル + コード 7 行 + callout 1（2 行まで）、または小見出し + リスト 5 項目。コードは、幅 500 の列で 1 行 33 字まで（Diff ありは 31 字、範囲の枠ありは 29 字）、幅 544 の列で 37 字まで（35 字、33 字）。
- 列の見出しは `h2`（30px）。列の中で `h1` を使わない。
- 図とコードを 1 つの画面に並べるときは、左右に並べ、片方にコードブロック、もう片方に図を置く（仕様の `content-guidelines/video.md`「図」）。コードと図を変えていく画面は、位置を決めて組む（README「ステージ」、見本は Camera）。コードの n 行目の中心は、コードブロックの上端から 32n + 4（`video-code-line`）。
