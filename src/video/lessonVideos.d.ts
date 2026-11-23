// レッスンごとの動画のコードを読み込む仮想モジュール。Vite のプラグイン（tools/lesson-videos.ts）が作る。
declare module "virtual:progrust-lesson-videos" {
  // レッスン ID の動画のコードを読み込む。動画のコードが無いレッスンでは、失敗する。
  export function loadLessonVideo(
    lessonId: string,
  ): Promise<import("./LessonVideo").LessonVideoCode>;
}
