表（`table`・`thead`・`tbody`・`tr`・`th`・`td`）で、縦の罫線を引かず、見出しの下に章カラーの 2px の線、行の間に細い線を引く。

## 書き方

```tsx
<table>
  <thead><tr><th>種類</th><th>書き方</th><th>同時に作れる数</th><th>値の書き換え</th></tr></thead>
  <tbody>
    <tr><td>不変参照</td><td><code>&T</code></td><td>いくつでも</td><td><strong className="c-ng">できない</strong></td></tr>
    <tr><td>可変参照</td><td><code>&mut T</code></td><td>1 つだけ</td><td><strong className="c-ok">できる</strong></td></tr>
  </tbody>
</table>
```

- 4 列・5 行（見出しを除く）まで。セルの文は 1 行に収める。
- 表の幅は内容に合わせて縮む（左寄せ）。最初の列の左と最後の列の右には余白を付けない。
- 見出し（`th`）は 18px・太さ 700・`ink-3`、セル（`td`）は 20px・`video-text`。見出しが 18px なので、1.12 倍以上に寄せた画面で読ませる（README「文字の最小サイズ」）。
- 型や式を並べる列のインラインコードの付け方は仕様の `content-guidelines/video.md`「画面の文章」 に従う（例の「書き方」の列）。
- 「できる／できない」のような正誤は `strong` に `c-ok` / `c-ng` を付けて示す（✓ / × が付く）。
- `thead` を必ず書く。`colspan`・`rowspan` は使わない。
- 1 つの画面に 1 つ。表から何を読み取ってほしいかを、ナレーションか、画面の文・callout で伝えることは仕様の `content-guidelines/video.md`「画面の文章」 に従う（preview では表の下の `p`）。
- 行は、読み上げに合わせて 1 行ずつ出してよい。
