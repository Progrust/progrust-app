---
type: Specification
title: "動画のシーンの書き方"
description: "シーンを React（TSX）の部品として書くときの決まり（`index.tsx` の default export、現在のフレームだけから絵を決めること、画像と図の扱い）と、共通部品（`useCues()`・`useLesson()`・`LessonTitle`・`CodeBlock`・`Camera`）を定める。"
---

シーンは React（TSX）の部品として書く。使える要素と部品、動き、レイアウトは制限しない。守ることは、この節の決まりと [動画でのコードの表示](code-display.md)〜[動画のコードの禁止事項](prohibited-code.md) とする。

`index.tsx` は、シーンの ID をキー、そのシーンを描く部品を値とするオブジェクトを default export する。

```tsx
// index.tsx（import は省略）
export default {
  title: TitleScene,
  move: MoveScene,
};
```

- `index.tsx` のキーは、台本の `scenes` の ID と過不足なく一致させる。
- アプリは、タイムライン（[動画のタイムラインと合図](timeline-and-cues.md)）に従ってシーンを順に表示し、音声を再生する。動画のコードには、シーンを並べる処理と音声を再生する処理を書かない。
- シーンの部品は、基準サイズ（1280×720）の領域に描く。props は受け取らない。
- シーンの中の現在の位置は、Remotion の `useCurrentFrame()` で得る（シーンの先頭が 0）。シーンの長さは、`useVideoConfig()` の `durationInFrames` で得る。
- 絵は、現在のフレームだけから決まるように書く。同じフレームは、通して再生したときも、再生位置を移した直後も、同じ絵になる。CSS のアニメーションとトランジション、タイマー、乱数は使わない。
- 見た目は、動画のデザインシステムのトークン（CSS 変数）と部品のクラス（`callout`・`mark` など）で指定する。
- レッスン専用の部品と CSS を、`video/` の中に作ってよい。CSS は普通の CSS で書く。CSS の効く範囲は [動画を構成するファイル](structure.md) に定める。
- 画像は、レッスンの `assets/` か全レッスン共通の `content/assets/` に置いたファイルを import して使う。
- 図（SVG）には `role="img"` と、図の要点を表す `aria-label` を付ける。画像には `alt` を付ける。

**共通部品**

全レッスンで使う部品を `src/video/` に用意する。動画のコードは、ここから import して使う。

| 部品 | 役割 |
|---|---|
| `useCues()` | 合図の関数 `at`・`endOf`（[動画のタイムラインと合図](timeline-and-cues.md)）を返す。シーンの部品の中で呼ぶ |
| `useLesson()` | アプリが指定した章とレッスン定義から、章の番号・章名・レッスンの番号・レッスン名を返す |
| `LessonTitle` | 表紙のレッスン名。`lesson.json` の `title` を表示する |
| `CodeBlock` | 台本の `code_blocks` の 1 件を表示する（[動画でのコードの表示](code-display.md)） |
| `Camera` | 寄せ。基準サイズの領域のうち、映す範囲（左上の位置と倍率）を指定する |
| 動きの補助 | 進み具合の計算、補間、要素の出現などの関数 |

- 最初のシーンは表紙とし、`LessonTitle` を 1 つ表示する。
- 章の番号・章名・レッスンの番号・レッスン名を画面に出すときは、`LessonTitle` か `useLesson()` を使う。動画のコードに書き写さない。章の番号とレッスンの番号は、同じ言語の章の中での位置と、言語ごとの表示の形（Rust は「第4章」「4-4」、C は「C 第1章」「C1-4」）に従う（[章とレッスンの並びの形式（chapters.json）](../chapters-json.md)「章とレッスンの番号」）。
- 寄せる倍率と動きの速さは、動画のデザインシステムに従う（[動画の作成ルール](../../content-guidelines/video.md)「動きと画面の量」）。

```tsx
// シーンの例（import は省略）
export const MoveScene = () => {
  const frame = useCurrentFrame();
  const { at } = useCues();
  // 文 m5 の開始から 0.5 秒後に、3 行目の s1 にマーカーを引く
  const marked = frame >= at("m5", 0.5) ? 1 : 0;
  return <CodeBlock block="main" marks={[{ line: 3, text: "s1", progress: marked }]} />;
};
```
