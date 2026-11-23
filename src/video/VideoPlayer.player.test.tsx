// VideoPlayer が Remotion の Player に渡す設定（documents/spec/content-format/video/structure.md）。
// Player を置き換えて、渡した props を確かめる。
import { render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { LessonVideoProps } from "./LessonVideo";
import { LessonVideo } from "./LessonVideo";
import { VideoPlayer } from "./VideoPlayer";

const received = vi.hoisted(() => ({ props: [] as Record<string, unknown>[] }));

vi.mock("@remotion/player", () => ({
  Player: (props: Record<string, unknown>) => {
    received.props.push(props);
    return null;
  },
}));

afterEach(() => {
  received.props = [];
});

const voice = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: {},
};

// 1 シーン・1 文。lead 0.9 秒 + 音声 2 秒 + tail 0.7 秒 = 108 フレーム。
const video: LessonVideoProps = {
  script: {
    scenes: [{ id: "s", name: "シーン" }],
    sentences: [{ id: "s1", scene: "s", text: "文", say: "ぶん" }],
  },
  durations: { s1: { seconds: 2.0, say: "ぶん", voice } },
  cueReference: {},
  scenes: { s: () => null },
  voiceDir: "/voice",
  chapters: [],
  lesson: {
    id: "ch01-01",
    chapter: "ch01",
    kind: "lesson",
    title: "レッスン",
    primary_elements: [],
    secondary_elements: [],
  },
};

it("Player の組み込みの操作を使わず、末尾で止め、16:9・30fps で動画を描く", () => {
  render(
    <VideoPlayer
      load={{ state: "ready", video }}
      chapterColor={1}
      lessonKind="lesson"
      initialFrame={42}
      initialSettings={{
        volume: 0.6,
        muted: true,
        playbackRate: 1.25,
        captions: false,
      }}
      endAction={null}
      onRetry={() => {}}
    />,
  );
  const props = received.props.at(-1);
  expect(props).toMatchObject({
    component: LessonVideo,
    inputProps: video,
    durationInFrames: 108,
    fps: 30,
    compositionWidth: 1280,
    compositionHeight: 720,
    initialFrame: 42,
    initialVolume: 0.6,
    initiallyMuted: true,
    playbackRate: 1.25,
    controls: false,
    spaceKeyToPlayOrPause: false,
    clickToPlay: false,
    doubleClickToFullscreen: false,
    moveToBeginningWhenEnded: false,
    loop: false,
    autoPlay: false,
  });
});
