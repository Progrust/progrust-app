図の説明（`figure.diagram > figcaption`）で、図の下に 18px・灰（`video-text-sub`）・中央寄せで出る。

## 書き方

```tsx
<figure className="diagram">
  <svg …>…</svg>
  <figcaption>図: <code>let b = Box::new(5);</code> のあとのメモリ</figcaption>
</figure>
```

- 「図: 」で始め、図が何の時点・どの状態を表すかを 1 行で書く。
- 図の要点（結論）は、ナレーションと画面の文・callout で伝え、`figcaption` には書かない。
- `figcaption` は図の外の文なので、コード片を `code` で囲んで書いてよい（仕様の `content-guidelines/video.md`「図」、囲む範囲は仕様の `content-guidelines/video.md`「画面の文章」）。図の `svg` の中にはコードを書かないので、どのコードの図かはここで示せる。
- 図とコードを左右に並べて変えていく画面では、どのコードの図かは隣のコードで分かるので、省いてよい。
- 理由: 図が「どのコードのどの時点か」を図の中の文字で書くと図がこみ入る。
