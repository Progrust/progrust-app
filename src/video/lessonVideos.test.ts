// アプリから、仮想モジュール virtual:progrust-lesson-videos（tools/lesson-videos.ts）を使えることを確かめる。
import { loadLessonVideo } from "virtual:progrust-lesson-videos";
import { expect, expectTypeOf, it } from "vitest";
import type { LessonVideoCode } from "./LessonVideo";

it("loadLessonVideo は、レッスンの動画のコードの型で返す", () => {
  expectTypeOf(loadLessonVideo).parameters.toEqualTypeOf<[string]>();
  expectTypeOf(
    loadLessonVideo,
  ).returns.resolves.toEqualTypeOf<LessonVideoCode>();
});

it("動画のコードが無いレッスン ID では、読み込みに失敗する", async () => {
  await expect(loadLessonVideo("no-such-lesson")).rejects.toThrow(
    "レッスン no-such-lesson の動画のコードが無い",
  );
});
