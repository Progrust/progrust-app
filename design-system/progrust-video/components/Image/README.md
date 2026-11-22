画像（`img`）で、角丸 16 と 2px の線（`video-rule`）で囲んで表示する。画像はテーマ（ライト／ダーク）で色が変わらない。

## 書き方

```tsx
import cargoRun from "../assets/cargo-run.png";

<img src={cargoRun} width={880} height={256} alt="cargo run を実行したターミナル" />
```

- 画像の置き場所（レッスンの `assets/` か、全レッスン共通の `content/assets/`）と、import して使うこと、`alt` を付けることは仕様の `content-format/video/scenes.md` に従う。外部の URL は参照できない（仕様の `content-format/video/prohibited-code.md`）。
- 使う場面: ターミナルやエディタの画面写しなど、実物を見せたいときだけ。仕組みの説明は画像にせず、SVG の図（`figure.diagram`）で描く。
- `width`・`height` に表示したい大きさ（基準の px）を書く。縮めて表示しない大きさで作り、画像の中の文字が 18px 以上になるようにする。
- ダークでも読めるように、画像は自分の地の色を持つもの（ターミナルの黒い地など）にする。透明の地に黒い線の画像は使わない。
- `alt` に画像の内容を 1 文で書く。エラーを指すときの書き方は仕様の `content-guidelines/video.md`「エラー」 に従う。
