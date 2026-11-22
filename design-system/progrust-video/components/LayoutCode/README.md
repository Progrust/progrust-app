コードを主役にする配置（`data-layout="code"`）で、見出し・コードブロック 1 つ・callout か本文を上から積む。コードブロックは内容の幅いっぱいになる。

## 書き方

```tsx
<div className="video-stage" data-layout="code">
  <h1>関数に渡すと、所有権が移る</h1>
  <CodeBlock block="main" />
  <p><code>takes_ownership(s)</code> を呼んだ時点で、s の所有権は引数の text に移ります。</p>
</div>
```

- コードは台本の `code_blocks` に書き、`CodeBlock` で表示する（仕様の `content-format/video/code-display.md`。CodeBlock）。
- コードの行数と字数は README「1 画面に載せる量」による。1.12〜1.2 倍に寄せて 11〜12 行、callout か本文を添えるなら 7〜8 行。寄せたときは、コードブロックの幅を内容の幅に合わせて狭める（`style` の `width`）。
- 上から積む配置なので、途中でブロックを足すと下のブロックが動く。callout や出力をあとから出すときは、位置を固定して組む（README「ステージ」）。
- 注目させたい字句はマーカー（CodeMark、同時に 1〜3 か所）。エラーの箇所は波線（CodeError）、変更は Diff（CodeDiff）。
- コンパイルエラーを見せるときは、コードの下に出力を置く（CompilerOutput、仕様の `content-guidelines/video.md`「エラー」）。コードと出力は合わせて 14 行まで。行番号を出力とそろえることは仕様の `content-guidelines/video.md`「コード」 に従う。
- コードの説明は、ナレーションのほかに、本文 1 つ（2 行まで）か callout 1 つ。
