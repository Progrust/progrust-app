import { describe, expect, it } from "vitest";
import {
  currentSentence,
  formatTime,
  hasReachedEnd,
  lastFrame,
  sceneMarkRatios,
  seekBarRatio,
  sentenceHeadFrame,
  sentenceStarts,
  skipTarget,
} from "./playerMath";
import type { Timeline } from "./timeline";

// 2 つのシーン。a は 0〜100、b は 100〜400（動画の長さは 400 フレーム）。
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

describe("再生中の文", () => {
  const sentences = sentenceStarts(timeline);

  it("文の開始位置を、動画の先頭からのフレームにして並べる", () => {
    expect(sentences).toEqual([
      { id: "a1", from: 27 },
      { id: "b1", from: 150 },
      { id: "b2", from: 230 },
    ]);
  });

  it("最初の文が始まる前は、最初の文", () => {
    expect(currentSentence(sentences, 0)?.id).toBe("a1");
    expect(currentSentence(sentences, 26)?.id).toBe("a1");
  });

  it("開始位置と同じ位置では、その文", () => {
    expect(currentSentence(sentences, 27)?.id).toBe("a1");
    expect(currentSentence(sentences, 150)?.id).toBe("b1");
  });

  it("開始位置より前の文のうち、最後の文", () => {
    expect(currentSentence(sentences, 149)?.id).toBe("a1");
    expect(currentSentence(sentences, 229)?.id).toBe("b1");
    expect(currentSentence(sentences, 399)?.id).toBe("b2");
  });

  it("文が無いときは undefined", () => {
    expect(currentSentence([], 10)).toBeUndefined();
  });

  it("文の頭のフレームは、文の開始位置", () => {
    expect(sentenceHeadFrame(sentences[1])).toBe(150);
  });
});

describe("5 秒の移動", () => {
  it("5 秒後・5 秒前に移す", () => {
    const long: Timeline = { ...timeline, durationInFrames: 3000 };
    expect(skipTarget(long, 600, "forward")).toBe(750);
    expect(skipTarget(long, 600, "back")).toBe(450);
  });

  it("先頭を越えるときは先頭、末尾を越えるときは末尾", () => {
    expect(skipTarget(timeline, 100, "back")).toBe(0);
    expect(skipTarget(timeline, 0, "back")).toBe(0);
    expect(skipTarget(timeline, 300, "forward")).toBe(399);
    expect(skipTarget(timeline, 399, "forward")).toBe(399);
  });
});

describe("シークバー", () => {
  it("再生位置を割合にする", () => {
    expect(seekBarRatio(timeline, 0)).toBe(0);
    expect(seekBarRatio(timeline, 100)).toBe(0.25);
    expect(seekBarRatio(timeline, 999)).toBe(1);
  });

  it("シーンの開始位置を印の割合にする", () => {
    expect(sceneMarkRatios(timeline)).toEqual([0, 0.25]);
  });

  it("長さが 0 のときは 0", () => {
    expect(seekBarRatio({ scenes: [], durationInFrames: 0 }, 0)).toBe(0);
  });
});

describe("時間の表示", () => {
  it("「0:42 / 1:44」の形にする", () => {
    expect(formatTime(42 * 30, 104 * 30)).toBe("0:42 / 1:44");
    expect(formatTime(0, 104 * 30)).toBe("0:00 / 1:44");
  });

  it("現在の位置は切り捨て、全体の長さは切り上げる", () => {
    expect(formatTime(59, 3175)).toBe("0:01 / 1:46");
    expect(formatTime(3174, 3175)).toBe("1:45 / 1:46");
  });
});

describe("末尾の判定", () => {
  it("最後のフレームは、長さ − 1", () => {
    expect(lastFrame(timeline)).toBe(399);
  });

  it("再生で最後のフレームに着いたときも、シークで末尾に移したときも真", () => {
    expect(hasReachedEnd(timeline, 399)).toBe(true);
    expect(hasReachedEnd(timeline, 400)).toBe(true);
    expect(hasReachedEnd(timeline, skipTarget(timeline, 380, "forward"))).toBe(
      true,
    );
  });

  it("末尾に着いていなければ偽", () => {
    expect(hasReachedEnd(timeline, 398)).toBe(false);
    expect(hasReachedEnd({ scenes: [], durationInFrames: 0 }, 0)).toBe(false);
  });
});
