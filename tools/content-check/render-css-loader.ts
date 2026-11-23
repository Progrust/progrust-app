// 教材チェックツールの描画（render.ts）で、Remotion のバンドラー（webpack）に足す CSS のローダー。
// アプリの Vite のプラグインと同じ変換を、css-loader より先に行う。
// - lesson: レッスン専用の CSS を、そのレッスンの動画の中に閉じ込める（lesson-css.ts、ADR 0093）。
// - font: デザインシステムの bundle.css から、書体を外部から読み込む `@import` を取り除く
//   （design-system-css.ts、ADR 0175）。
import { stripFontImport } from "../design-system-css.ts";
import { scopeLessonCss } from "../lesson-css.ts";

export type RenderCssLoaderOptions =
  { mode: "lesson"; lessonId: string } | { mode: "font" };

interface LoaderContext {
  getOptions(): RenderCssLoaderOptions;
}

export default function renderCssLoader(
  this: LoaderContext,
  source: string,
): string {
  const options = this.getOptions();
  return options.mode === "lesson"
    ? scopeLessonCss(source, options.lessonId)
    : stripFontImport(source);
}
