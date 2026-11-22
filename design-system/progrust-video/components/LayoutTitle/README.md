レッスンの表紙にする配置（`data-layout="title"`）で、レッスン名（共通部品 `LessonTitle`）とサブタイトルだけを置く。動画の最初のシーンに使う。

## 書き方

```tsx
<div className="video-stage" data-layout="title">
  <LessonTitle />
  <p>代入で、値の持ち主が移る仕組み</p>
</div>
```

- **書き方は仕様書に従う**。最初のシーンを表紙にして `LessonTitle` を 1 つ表示すること、レッスン名を動画のコードに書き写さないことは仕様の `content-format/video/scenes.md`「共通部品」 と `content-guidelines/video.md`「構成と長さ」、サブタイトルに句点を付けないことは仕様の `content-guidelines/video.md`「画面の文章」 を唯一の定義とする。
- `LessonTitle` は、レッスン名（`lesson.json` の `title`）を見出し（`h1`）として出す。2 行まで、1 行 12 字が目安（`video-display` 64px、幅 784px）。
- サブタイトル（`p`）は 1 文・1 行（27 字まで）。レッスンで何が分かるようになるかを書く。
- 表紙は寄せない（1 倍。README「寄せ方」）。
- 右側の章カラーの飾り（縦長の塗り `video-primary` と、ずらした淡い塗り `video-accent-fill`）はテーマ CSS が描く。
- ステージに `data-chapter-label`（例「第4章 所有権」）を付けると、レッスン名の上に章の名前が出る。値は `useLesson()` から得る（RunningHead）。

## 動かし方

- 飾りを動かすときは、ステージに `deco-as-elements` を付けて CSS の飾り（`::after`）を消し、要素で描く。位置・大きさ・色は CSS が決める（`::after` の飾りと同じ）。動かすのは `translate`・`opacity` と、棒の伸び具合（`--bar-p`）。

```tsx
<div className="video-stage deco-as-elements" data-layout="title">
  <div className="title-deco-soft" />
  <div className="title-deco">
    <span className="title-deco-bar" /><span className="title-deco-bar" /><span className="title-deco-bar" />
  </div>
  <LessonTitle />
  <p>代入で、値の持ち主が移る仕組み</p>
</div>
```

- 現れ方の目安（表紙は、ほかの画面より長く、大きく動かす）:
  - 章カラーの板は、上から 26 フレームで下りてくる。淡い板は 4 フレーム遅れて、下から 120px 上がりながら 26 フレームで現れる。
  - 板の中の 3 本の棒は、18 フレーム目から 4 フレームずつずらして、12 フレームで左から伸びる。
  - レッスン名は 10 フレーム目から 20 フレームで、下から 28px 上がりながら現れる。サブタイトルは 18 フレーム目から 20 フレームで、20px。
- 表示のあいだ、2 つの板をごくゆっくり（シーン全体で 10px）上下に動かしてよい。飾りの動きを使うのは、この表紙だけ（README「動きの種類と速さ」）。
- シーンの出入りは 10 フレームのフェード。

## してはいけないこと

- レッスン名とサブタイトル以外の要素（コード、図、リスト）を置かない。
- 色クラスを付けない。
