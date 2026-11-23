import { describe, expect, it } from "vitest";
import type { LessonProgress } from "../commands/progress";
import {
  initialFrame,
  rightAction,
  savePositionArgs,
} from "./videoScreenLogic";
import type { Timeline } from "./timeline";

// 文は a1（27）・b1（150）・b2（230、最後の文）。
const timeline: Timeline = {
  durationInFrames: 400,
  scenes: [
    {
      id: "a",
      from: 0,
      durationInFrames: 100,
      sentences: [{ id: "a1", from: 27, audioEnd: 60, ratio: 1 }],
    },
    {
      id: "b",
      from: 100,
      durationInFrames: 300,
      sentences: [
        { id: "b1", from: 50, audioEnd: 120, ratio: 1 },
        { id: "b2", from: 130, audioEnd: 200, ratio: 1 },
      ],
    },
  ],
};

function progress(overrides: Partial<LessonProgress> = {}): LessonProgress {
  return {
    status: "in_progress",
    first_completed_at: null,
    last_completed_at: null,
    completion_count: 0,
    last_opened_at: "2026-11-23T10:00:00+09:00",
    last_sentence_id: null,
    video_watched: false,
    last_passed_session_id: null,
    ...overrides,
  };
}

describe("開いたときの位置", () => {
  it("途中の文で離れていたら、その文の頭", () => {
    expect(initialFrame(timeline, "b1")).toBe(150);
    expect(initialFrame(timeline, "a1")).toBe(27);
  });

  it("まだ再生していない（null）ときは、動画の先頭", () => {
    expect(initialFrame(timeline, null)).toBe(0);
  });

  it("最後の文のときは、動画の先頭（最後まで再生したあと）", () => {
    expect(initialFrame(timeline, "b2")).toBe(0);
  });

  it("台本が変わって、その文がなくなっていても、エラーにならず動画の先頭", () => {
    expect(initialFrame(timeline, "gone")).toBe(0);
  });

  it("文が 1 つもない台本でも、動画の先頭", () => {
    expect(initialFrame({ durationInFrames: 0, scenes: [] }, "a1")).toBe(0);
  });
});

describe("右端の行動", () => {
  it("最後まで再生するまでは、「演習へ」を出さない", () => {
    expect(rightAction("lesson", undefined, false, true)).toBeNull();
    expect(rightAction("lesson", progress(), false, true)).toBeNull();
  });

  it("この画面で末尾に着いたら（シークを含む）、「演習へ」を出す", () => {
    expect(rightAction("lesson", progress(), true, true)).toBe("exercise");
  });

  it("開き直しても、保存した video_watched で「演習へ」を出し続ける", () => {
    expect(
      rightAction("lesson", progress({ video_watched: true }), false, true),
    ).toBe("exercise");
  });

  it("完了済みのレッスンは、開いた時点から「演習へ」を出す", () => {
    expect(
      rightAction("lesson", progress({ status: "completed" }), false, true),
    ).toBe("exercise");
  });

  it("「演習へ」は、次のレッスンの有無によらない", () => {
    expect(
      rightAction("lesson", progress({ video_watched: true }), false, false),
    ).toBe("exercise");
  });

  it("動画のみのレッスンでは、「演習へ」を出さない", () => {
    expect(rightAction("video_only", progress(), true, true)).not.toBe(
      "exercise",
    );
    expect(
      rightAction(
        "video_only",
        progress({ status: "completed", video_watched: true }),
        false,
        true,
      ),
    ).not.toBe("exercise");
  });

  it("動画のみのレッスンは、最後まで再生するまで「次のレッスンへ」を出さない", () => {
    expect(rightAction("video_only", progress(), false, true)).toBeNull();
    expect(rightAction("video_only", undefined, false, true)).toBeNull();
  });

  it("動画のみのレッスンは、最後まで再生すると「次のレッスンへ」を出す", () => {
    expect(rightAction("video_only", progress(), true, true)).toBe(
      "next_lesson",
    );
  });

  it("動画のみのレッスンは、開き直しても「次のレッスンへ」を出し続ける", () => {
    expect(
      rightAction(
        "video_only",
        progress({ status: "completed", video_watched: true }),
        false,
        true,
      ),
    ).toBe("next_lesson");
  });

  it("動画のみのレッスンで、次のレッスンが無いときは、何も出さない", () => {
    expect(rightAction("video_only", progress(), true, false)).toBeNull();
  });

  it("動画の無い種類では、何も出さない", () => {
    expect(
      rightAction(
        "chapter_review",
        progress({ video_watched: true }),
        true,
        true,
      ),
    ).toBeNull();
    expect(rightAction("generated", progress(), true, true)).toBeNull();
  });
});

describe("再生位置を保存するきっかけ", () => {
  it("再生中の文が変わったとき・一時停止したとき・画面を離れるときは、video_watched が false", () => {
    for (const trigger of ["sentence_changed", "paused", "leave"] as const) {
      expect(savePositionArgs("ch01-01", trigger, "b1", false)).toEqual({
        lesson_id: "ch01-01",
        last_sentence_id: "b1",
        video_watched: false,
      });
    }
  });

  it("最後まで再生したときは、video_watched が true", () => {
    expect(savePositionArgs("ch01-01", "ended", "b2", true)).toEqual({
      lesson_id: "ch01-01",
      last_sentence_id: "b2",
      video_watched: true,
    });
  });

  it("末尾に着いたあとに画面を離れるときも、video_watched が true", () => {
    expect(savePositionArgs("ch01-01", "leave", "b2", true)).toEqual({
      lesson_id: "ch01-01",
      last_sentence_id: "b2",
      video_watched: true,
    });
  });

  it("再生中の文が無いときは、保存しない", () => {
    expect(savePositionArgs("ch01-01", "leave", undefined, false)).toBeNull();
  });
});
