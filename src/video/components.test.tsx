import { Player } from "@remotion/player";
import { clearMocks, mockConvertFileSrc } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import { useCurrentFrame } from "remotion";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Chapter, Script } from "../commands/content";
import { Camera, cameraAt } from "./Camera";
import { LessonTitle } from "./LessonTitle";
import { LessonVideo, type LessonVideoProps } from "./LessonVideo";
import { computeTimeline, type Durations } from "./timeline";

beforeEach(() => {
  mockConvertFileSrc("macos");
  vi.stubEnv("NODE_ENV", "development");
});

afterEach(() => {
  clearMocks();
  vi.unstubAllEnvs();
});

const voice = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: {},
};

const script: Script = {
  code_blocks: [],
  scenes: [{ id: "title", name: "表紙", lead: 1.0, tail: 0.5 }],
  sentences: [{ id: "t1", scene: "title", text: "表紙", say: "ひょうし" }],
};
const durations: Durations = {
  t1: { seconds: 2.0, say: "ひょうし", voice },
};
const chapters: Chapter[] = [
  {
    id: "ch04",
    title: "所有権",
    description: "",
    lessons: [
      {
        id: "ch04-01",
        kind: "lesson",
        title: "所有権のムーブ",
        summary: null,
        estimated_minutes: null,
        in_preparation: false,
      },
    ],
  },
];

function Cover() {
  const frame = useCurrentFrame();
  const view = cameraAt(
    frame,
    { x: 0, y: 0, scale: 1 },
    { x: 22, y: 0, scale: 1.16 },
    10,
  );
  return (
    <Camera {...view}>
      <div className="video-stage" data-layout="title">
        <LessonTitle />
      </div>
    </Camera>
  );
}

function renderAt(frame: number, title = "所有権のムーブ") {
  const props: LessonVideoProps = {
    script,
    durations,
    cueReference: {},
    scenes: { title: Cover },
    voiceDir: "/voice",
    chapters,
    lesson: {
      id: "ch04-01",
      chapter: "ch04",
      kind: "lesson",
      title,
      primary_elements: [],
      secondary_elements: [],
    },
  };
  const timeline = computeTimeline(script, durations, {});
  return render(
    <Player
      component={LessonVideo}
      inputProps={props}
      durationInFrames={timeline.durationInFrames}
      fps={30}
      compositionWidth={1280}
      compositionHeight={720}
      initialFrame={frame}
      acknowledgeRemotionLicense
      errorFallback={({ error }) => <p role="alert">{error.message}</p>}
    />,
  );
}

it("LessonTitle は、lesson.json の title を、h1 として表示する", () => {
  renderAt(0, "借用と参照");
  expect(
    screen.getByRole("heading", { level: 1, name: "借用と参照" }),
  ).toBeInTheDocument();
});

it("Camera は、映す範囲を .video-camera の CSS 変数で渡す", () => {
  const { container } = render(
    <Camera x={22} y={0} scale={1.16}>
      <div className="video-stage" />
    </Camera>,
  );
  const camera = container.querySelector<HTMLElement>(".video-camera");
  expect(camera?.style.getPropertyValue("--camera-x")).toBe("22");
  expect(camera?.style.getPropertyValue("--camera-y")).toBe("0");
  expect(camera?.style.getPropertyValue("--camera-scale")).toBe("1.16");
  expect(camera?.querySelector(".video-stage")).not.toBeNull();
});

it("Camera は、省くと 1 倍（寄せない）になる", () => {
  const { container } = render(<Camera>x</Camera>);
  const camera = container.querySelector<HTMLElement>(".video-camera");
  expect(camera?.style.getPropertyValue("--camera-scale")).toBe("1");
});

it("同じフレームは、再生位置をどう移しても、同じ絵になる", () => {
  const html = (frame: number) => {
    const { container, unmount } = renderAt(frame);
    const result = container.querySelector(".video-camera")?.outerHTML;
    unmount();
    return result;
  };
  const first = html(20);
  html(5);
  html(60);
  expect(html(20)).toBe(first);
  expect(html(5)).not.toBe(first);
});
