import { describe, expect, it } from "vitest";
import { exerciseLinks } from "./exerciseLinks";

describe("exerciseLinks", () => {
  it("レッスンは「レッスン一覧」で、動画を見直せる", () => {
    expect(exerciseLinks("lesson", "ch01-01")).toEqual({
      backLabel: "レッスン一覧",
      backRoute: { name: "lessons" },
      reviewVideoRoute: { name: "video", lessonId: "ch01-01" },
    });
  });

  it("章末復習レッスンでは「動画を見直す」を表示しない", () => {
    expect(exerciseLinks("chapter_review", "ch01-99")).toEqual({
      backLabel: "レッスン一覧",
      backRoute: { name: "lessons" },
      reviewVideoRoute: null,
    });
  });

  it("生成問題は「生成問題一覧」で、動画を見直せない", () => {
    expect(exerciseLinks("generated", "gen-1")).toEqual({
      backLabel: "生成問題一覧",
      backRoute: { name: "generated" },
      reviewVideoRoute: null,
    });
  });
});
