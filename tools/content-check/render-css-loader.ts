// 教材チェックツールの描画（render.ts）で、Remotion のバンドラー（webpack）に足す CSS のローダー。
// アプリの Vite のプラグインと同じ変換を、css-loader より先に行う。
// レッスン専用の CSS を、そのレッスンの動画の中に閉じ込める（lesson-css.ts、ADR 0093）。
import { scopeLessonCss } from "../lesson-css.ts";

export type RenderCssLoaderOptions = { lessonId: string };

interface LoaderContext {
  getOptions(): RenderCssLoaderOptions;
}

export default function renderCssLoader(
  this: LoaderContext,
  source: string,
): string {
  return scopeLessonCss(source, this.getOptions().lessonId);
}
