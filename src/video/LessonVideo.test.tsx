import { Player, type PlayerRef } from "@remotion/player";
import { clearMocks, mockConvertFileSrc } from "@tauri-apps/api/mocks";
import { act, render, screen } from "@testing-library/react";
import { createRef, type Ref } from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Chapter, Script } from "../commands/content";
import { useCodeBlocks, useCues, useLesson } from "./hooks";
import {
  LessonVideo,
  type LessonVideoProps,
  type SceneComponents,
} from "./LessonVideo";
import { computeTimeline, type Durations } from "./timeline";

beforeEach(() => {
  mockConvertFileSrc("macos");
  // remotion 4.0.545 は、NODE_ENV が test のとき、Html5Audio をタイムラインに登録しようとし、
  // Player が登録先を用意していないので、描画時にエラーになる（アプリでは NODE_ENV が test にならない）。
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

// テスト用の台本。title: lead 1.0 秒・tail 0.5 秒、move: lead 0.5 秒・tail 1.0 秒。
const script: Script = {
  code_blocks: [
    {
      id: "main",
      label: "src/main.rs",
      language: "rust",
      code: "fn main() {}",
    },
  ],
  scenes: [
    { id: "title", name: "表紙", lead: 1.0, tail: 0.5 },
    { id: "move", name: "ムーブ", lead: 0.5, tail: 1.0 },
  ],
  sentences: [
    { id: "t1", scene: "title", text: "表紙", say: "ひょうし" },
    {
      id: "m1",
      scene: "move",
      text: "文 1",
      say: "ぶんいち",
      pause_after: 0.5,
    },
    { id: "m2", scene: "move", text: "文 2", say: "ぶんに" },
  ],
};

// title: t1 30〜90、長さ 105。move（105 から）: m1 15〜75、m2 90〜150、長さ 180。
const durations: Durations = {
  t1: { seconds: 2.0, say: "ひょうし", voice },
  m1: { seconds: 2.0, say: "ぶんいち", voice },
  m2: { seconds: 2.0, say: "ぶんに", voice },
};

const chapters: Chapter[] = [
  {
    id: "ch01",
    title: "はじめに",
    description: "",
    language: "rust",
    number: 1,
    lessons: [],
  },
  {
    id: "ch04",
    title: "所有権",
    description: "",
    language: "rust",
    number: 4,
    lessons: ["ch04-01", "ch04-02"].map((id) => ({
      id,
      kind: "lesson",
      title: id,
      summary: null,
      estimated_minutes: null,
      in_preparation: false,
    })),
  },
];

// シーンの中で、フレーム・シーンの長さ・合図・レッスンの情報・コードブロックを表示する。
function makeScene(name: string, cueSentence: string) {
  return function Scene() {
    const frame = useCurrentFrame();
    const { durationInFrames } = useVideoConfig();
    const { at, endOf } = useCues();
    const lesson = useLesson();
    const blocks = useCodeBlocks();
    return (
      <dl>
        <dt>scene</dt>
        <dd>{name}</dd>
        <dt>frame</dt>
        <dd>{frame}</dd>
        <dt>duration</dt>
        <dd>{durationInFrames}</dd>
        <dt>at</dt>
        <dd>{at(cueSentence, 1.0)}</dd>
        <dt>endOf</dt>
        <dd>{endOf(cueSentence)}</dd>
        <dt>lesson</dt>
        <dd>
          {lesson.chapterNumber} {lesson.chapterTitle} {lesson.lessonNumber}{" "}
          {lesson.lessonTitle}
        </dd>
        <dt>blocks</dt>
        <dd>{blocks.map((b) => b.id).join(",")}</dd>
      </dl>
    );
  };
}

const scenes: SceneComponents = {
  title: makeScene("title", "t1"),
  move: makeScene("move", "m2"),
};

function props(overrides: Partial<LessonVideoProps> = {}): LessonVideoProps {
  return {
    script,
    durations,
    cueReference: {},
    scenes,
    voiceDir: "/app/Resources/content/lessons/ch04-02/video/voice",
    chapters,
    lesson: {
      id: "ch04-02",
      chapter: "ch04",
      kind: "lesson",
      title: "所有権のムーブ",
      primary_elements: [],
      secondary_elements: [],
    },
    ...overrides,
  };
}

function renderAt(
  frame: number,
  inputProps: LessonVideoProps = props(),
  ref?: Ref<PlayerRef>,
) {
  const timeline = computeTimeline(
    inputProps.script,
    inputProps.durations,
    inputProps.cueReference,
  );
  return render(
    <Player
      ref={ref}
      component={LessonVideo}
      inputProps={inputProps}
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

// 表示した項目の値。
function value(term: string): string | null {
  const dt = screen.getByText(term, { selector: "dt" });
  return dt.nextElementSibling?.textContent ?? null;
}

// Player が共有の <audio> に入れた音声の URL。
function audioSources(container: HTMLElement): string[] {
  return Array.from(container.querySelectorAll("audio"))
    .map((audio) => audio.src)
    .filter((src) => !src.startsWith("data:"));
}

it("シーンを台本の順に並べ、シーンの部品にシーンの中のフレームと長さを渡す", () => {
  renderAt(40);
  expect(value("scene")).toBe("title");
  expect(value("frame")).toBe("40");
  expect(value("duration")).toBe("105");
});

it("2 つ目のシーンでは、シーンの先頭を 0 としたフレームと合図を使う", () => {
  renderAt(105 + 20);
  expect(value("scene")).toBe("move");
  expect(value("frame")).toBe("20");
  expect(value("duration")).toBe("180");
  // m2 の開始 90 から 1.0 秒後。
  expect(value("at")).toBe("120");
  expect(value("endOf")).toBe("150");
});

it("cue-reference.json の長さとの比を、合図の正の秒に掛ける", () => {
  // m2 の合図は 4.0 秒の音声で合わせ、いまの音声は 2.0 秒。1.0 秒は 0.5 秒後になる。
  renderAt(105 + 20, props({ cueReference: { m2: 4.0 } }));
  expect(value("at")).toBe(String(90 + 15));
});

it("シーンを、レッスン ID の data-lesson を持つ要素の中に描く", () => {
  const { container } = renderAt(40);
  const root = container.querySelector('[data-lesson="ch04-02"]');
  expect(root).not.toBeNull();
  expect(root).toContainElement(screen.getByText("title", { selector: "dd" }));
  expect(container.querySelectorAll("[data-lesson]")).toHaveLength(1);
});

it("useLesson() は、章の並びとレッスン定義から決めた番号と名前を返す", () => {
  renderAt(40);
  expect(value("lesson")).toBe("2 所有権 2-2 所有権のムーブ");
});

it("useCodeBlocks() は、台本の code_blocks を返す", () => {
  renderAt(40);
  expect(value("blocks")).toBe("main");
});

it("文の音声を、文の開始位置から、voice_dir と文の ID の asset:// の URL で鳴らす", () => {
  const url = (id: string) =>
    `asset://localhost/${encodeURIComponent(
      `/app/Resources/content/lessons/ch04-02/video/voice/${id}.m4a`,
    )}`;

  const player = createRef<PlayerRef>();
  const { container } = renderAt(0, props(), player);
  // Player は、共有の <audio> に URL を入れるのを、描き直したときに行うので、シークして確かめる。
  const seek = (frame: number) => act(() => player.current?.seekTo(frame));

  // t1 は 30〜90。始まる前は鳴らさない。
  seek(29);
  expect(audioSources(container)).toEqual([]);
  seek(30);
  expect(audioSources(container)).toEqual([url("t1")]);
  seek(89);
  expect(audioSources(container)).toEqual([url("t1")]);
  // 音声の終わり（90）から、次のシーンの m1 の開始（105 + 15）までは鳴らさない。
  seek(90);
  expect(audioSources(container)).toEqual([]);
  seek(105 + 15);
  expect(audioSources(container)).toEqual([url("m1")]);
  // m2 は 105 + 90 から。
  seek(105 + 100);
  expect(audioSources(container)).toEqual([url("m2")]);
});

it("合図で、ほかのシーンの文の ID を指すと、描画時にエラーになる", () => {
  renderAt(
    40,
    props({ scenes: { ...scenes, title: makeScene("title", "m1") } }),
  );
  expect(screen.getByRole("alert")).toHaveTextContent(
    "シーン title の合図が、ほかのシーン move の文 m1 を指している",
  );
});

it("合図で、台本にない文の ID を指すと、描画時にエラーになる", () => {
  renderAt(
    40,
    props({ scenes: { ...scenes, title: makeScene("title", "x9") } }),
  );
  expect(screen.getByRole("alert")).toHaveTextContent(
    "シーン title の合図が、台本にない文 x9 を指している",
  );
});

it("index.tsx にシーンの部品が無いと、描画時にエラーになる", () => {
  renderAt(40, props({ scenes: { title: scenes.title } }));
  expect(screen.getByRole("alert")).toHaveTextContent(
    "index.tsx に、シーン move の部品がない",
  );
});
