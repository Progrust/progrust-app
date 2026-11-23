import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Chapter, Script } from "../commands/content";
import { NavigationContext } from "../router";
import type { Route } from "../router";
import type { Durations } from "./timeline";
import { VideoScreen } from "./VideoScreen";

const loadLessonVideo = vi.hoisted(() => vi.fn());
vi.mock("virtual:progrust-lesson-videos", () => ({ loadLessonVideo }));

const voice = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: {},
};
const script: Script = {
  scenes: [{ id: "s", name: "場面", lead: 1.0, tail: 0.5 }],
  sentences: [
    { id: "a1", scene: "s", text: "一つ目", say: "いち" },
    { id: "a2", scene: "s", text: "二つ目", say: "に" },
    { id: "a3", scene: "s", text: "三つ目", say: "さん" },
  ],
};
// 1.0 秒の lead のあと、a1 は 30 フレーム、a2 は 60 フレーム付近から。
const durations: Durations = {
  a1: { seconds: 1.0, say: "いち", voice },
  a2: { seconds: 1.0, say: "に", voice },
  a3: { seconds: 1.0, say: "さん", voice },
};

const chapters: Chapter[] = [
  { id: "ch01", title: "はじめに", description: "", lessons: [] },
  { id: "ch02", title: "変数", description: "", lessons: [] },
  { id: "ch03", title: "型", description: "", lessons: [] },
  {
    id: "ch04",
    title: "所有権",
    description: "",
    lessons: [
      lesson("ch04-01", "video_only", "メモリの仕組み"),
      lesson("ch04-02", "lesson", "所有権のムーブ"),
      lesson("ch04-03", "lesson", "借用"),
    ],
  },
];

function lesson(id: string, kind: "lesson" | "video_only", title: string) {
  return {
    id,
    kind,
    title,
    summary: null,
    estimated_minutes: null,
    in_preparation: false,
  };
}

const baseProgress: {
  status: string;
  first_completed_at: string | null;
  last_completed_at: string | null;
  completion_count: number;
  last_opened_at: string;
  last_sentence_id: string | null;
  video_watched: boolean;
  last_passed_session_id: string | null;
} = {
  status: "in_progress",
  first_completed_at: null,
  last_completed_at: null,
  completion_count: 0,
  last_opened_at: "2026-11-01T00:00:00+09:00",
  last_sentence_id: null,
  video_watched: false,
  last_passed_session_id: null,
};

const baseSettings = {
  schema_version: 1,
  theme: "system",
  editor_font_size: 14,
  video: { volume: 0.5, muted: false, playback_rate: 1.5, captions: true },
  models: { chat: "sonnet", background: "opus" },
  tool_paths: { cargo: null, claude: null },
  first_launch_completed: true,
};

type Calls = { command: string; args: unknown }[];

function setup(
  lessonId: string,
  options: {
    progress?: Partial<typeof baseProgress>;
    kind?: "lesson" | "video_only";
    failLesson?: boolean;
  } = {},
) {
  const kind =
    options.kind ?? (lessonId === "ch04-01" ? "video_only" : "lesson");
  const calls: Calls = [];
  let lessonFailures = options.failLesson === true ? 1 : 0;
  mockIPC((command, args) => {
    calls.push({ command, args });
    switch (command) {
      case "list_chapters":
        return { chapters };
      case "get_lesson":
        if (lessonFailures > 0) {
          lessonFailures -= 1;
          throw new Error("読み込めない");
        }
        return {
          lesson: {
            id: lessonId,
            chapter: "ch04",
            kind,
            title: "所有権のムーブ",
            primary_elements: [],
            secondary_elements: [],
          },
          script,
          voice_dir: "/voice",
          exercise: null,
        };
      case "open_lesson":
        return {
          session_id: null,
          progress: { ...baseProgress, ...options.progress },
          generated: null,
          draft: null,
        };
      case "get_settings":
        return baseSettings;
      case "update_settings":
        return (args as { settings: unknown }).settings;
      case "save_video_position":
        return baseProgress;
      default:
        return undefined;
    }
  });
  const navigate = vi.fn<(route: Route) => void>();
  const view = render(
    <NavigationContext value={{ route: { name: "video", lessonId }, navigate }}>
      <VideoScreen lessonId={lessonId} />
    </NavigationContext>,
  );
  return { calls, navigate, ...view };
}

function commands(calls: Calls, name: string) {
  return calls.filter((c) => c.command === name).map((c) => c.args);
}

beforeEach(() => {
  mockConvertFileSrc("macos");
  vi.stubEnv("NODE_ENV", "development");
  vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
  vi.spyOn(HTMLMediaElement.prototype, "pause").mockReturnValue(undefined);
  loadLessonVideo.mockReset();
  loadLessonVideo.mockResolvedValue({
    scenes: { s: () => <p>場面</p> },
    durations,
    cueReference: {},
  });
});

afterEach(() => {
  clearMocks();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

async function ready() {
  await screen.findByRole("button", { name: "再生" });
}

describe("開いたとき", () => {
  it("文脈の行に、戻るリンク・章のチップ・レッスンの番号と名前を出し、章カラーを付ける", async () => {
    const { container } = setup("ch04-02");
    await ready();
    expect(screen.getByRole("link", { name: "レッスン一覧" })).toBeVisible();
    const chip = container.querySelector(".pg-chip");
    expect(chip).toHaveTextContent("4所有権");
    expect(container.querySelector(".pg-context-title")).toHaveTextContent(
      "4-2 所有権のムーブ",
    );
    expect(container.querySelector("main")).toHaveAttribute(
      "data-chapter",
      "4",
    );
    expect(container.querySelector(".pg-video")).toHaveAttribute(
      "data-chapter",
      "4",
    );
  });

  it("open_lesson を呼び、保存した文の頭の位置で止めて表示する", async () => {
    const { calls, container } = setup("ch04-02", {
      progress: { last_sentence_id: "a2" },
    });
    await ready();
    expect(commands(calls, "open_lesson")).toEqual([{ lesson_id: "ch04-02" }]);
    expect(commands(calls, "get_lesson")).toEqual([{ lesson_id: "ch04-02" }]);
    expect(loadLessonVideo).toHaveBeenCalledWith("ch04-02");
    expect(container.querySelector(".pg-video-time b")).toHaveTextContent(
      "0:02",
    );
  });

  it("保存してある設定（字幕の表示など）で表示する", async () => {
    const { container } = setup("ch04-02");
    await ready();
    expect(screen.getByRole("button", { name: "字幕" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("slider", { name: "音量" })).toHaveAttribute(
      "aria-valuenow",
      "50",
    );
    expect(container.querySelector(".pg-captions")).toHaveTextContent("一つ目");
  });

  it("読み込み中は回る印を出し、失敗したら「もう一度読み込む」で読み込み直す", async () => {
    const { calls } = setup("ch04-02", { failLesson: true });
    expect(screen.getByRole("status")).toBeInTheDocument();
    await screen.findByText("動画を読み込めませんでした");
    fireEvent.click(screen.getByRole("button", { name: "もう一度読み込む" }));
    await ready();
    expect(commands(calls, "get_lesson")).toHaveLength(2);
    // 開いたことの記録は、読み直しでは増やさない。
    expect(commands(calls, "open_lesson")).toHaveLength(1);
  });

  it("動画のコードの読み込みに失敗したときも、失敗を表示する", async () => {
    loadLessonVideo.mockRejectedValue(new Error("動画のコードが無い"));
    setup("ch04-02");
    await screen.findByText("動画を読み込めませんでした");
    expect(screen.getByRole("link", { name: "レッスン一覧" })).toBeVisible();
  });
});

describe("右端の行動", () => {
  it("動画を見ていないレッスンでは出さない", async () => {
    setup("ch04-02");
    await ready();
    expect(
      screen.queryByRole("button", { name: "演習へ" }),
    ).not.toBeInTheDocument();
  });

  it("動画を最後まで見たレッスンでは「演習へ」で演習画面に移る", async () => {
    const { navigate } = setup("ch04-02", {
      progress: { video_watched: true },
    });
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "演習へ" }));
    expect(navigate).toHaveBeenCalledWith({
      name: "exercise",
      lessonId: "ch04-02",
    });
  });

  it("動画のみのレッスンでは、見たあと「次のレッスンへ」で次のレッスンの動画画面に移る", async () => {
    const { navigate } = setup("ch04-01", {
      progress: { video_watched: true },
    });
    await ready();
    expect(
      screen.queryByRole("button", { name: "演習へ" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "次のレッスンへ" }));
    expect(navigate).toHaveBeenCalledWith({
      name: "video",
      lessonId: "ch04-02",
    });
  });
});

describe("読み込みに失敗したときの右端の行動", () => {
  it("動画のコードの読み込みに失敗しても、見終わったレッスンでは「演習へ」で進める", async () => {
    loadLessonVideo.mockRejectedValue(new Error("動画のコードが無い"));
    const { navigate } = setup("ch04-02", {
      progress: { video_watched: true },
    });
    await screen.findByText("動画を読み込めませんでした");
    fireEvent.click(screen.getByRole("button", { name: "演習へ" }));
    expect(navigate).toHaveBeenCalledWith({
      name: "exercise",
      lessonId: "ch04-02",
    });
  });

  it("動画のみのレッスンでも、失敗したまま「次のレッスンへ」で進める", async () => {
    loadLessonVideo.mockRejectedValue(new Error("動画のコードが無い"));
    const { navigate } = setup("ch04-01", {
      progress: { video_watched: true },
    });
    await screen.findByText("動画を読み込めませんでした");
    fireEvent.click(screen.getByRole("button", { name: "次のレッスンへ" }));
    expect(navigate).toHaveBeenCalledWith({
      name: "video",
      lessonId: "ch04-02",
    });
  });

  it("見ていないレッスンでは、失敗しても出さない", async () => {
    loadLessonVideo.mockRejectedValue(new Error("動画のコードが無い"));
    setup("ch04-02");
    await screen.findByText("動画を読み込めませんでした");
    expect(
      screen.queryByRole("button", { name: "演習へ" }),
    ).not.toBeInTheDocument();
  });
});

describe("再生位置の保存", () => {
  it("レッスン一覧に戻るとき、いまの位置と、見終わっていないことを保存する", async () => {
    const { calls, navigate, unmount } = setup("ch04-02", {
      progress: { last_sentence_id: "a2" },
    });
    await ready();
    fireEvent.click(screen.getByRole("link", { name: "レッスン一覧" }));
    expect(navigate).toHaveBeenCalledWith({ name: "lessons" });
    unmount();
    expect(commands(calls, "save_video_position")).toEqual([
      { lesson_id: "ch04-02", last_sentence_id: "a2", video_watched: false },
    ]);
  });

  it("末尾へ移すと、その文を、見終わったこととして保存する", async () => {
    const { calls, unmount } = setup("ch04-02");
    await ready();
    const bar = screen.getByRole("slider", { name: "再生位置" });
    vi.spyOn(bar, "getBoundingClientRect").mockReturnValue(
      DOMRect.fromRect({ x: 0, y: 0, width: 1000, height: 20 }),
    );
    await act(async () => {
      fireEvent.pointerDown(bar, { button: 0, pointerId: 1, clientX: 1000 });
      fireEvent.pointerUp(bar, { button: 0, pointerId: 1, clientX: 1000 });
    });
    await waitFor(() =>
      expect(commands(calls, "save_video_position")).toContainEqual({
        lesson_id: "ch04-02",
        last_sentence_id: "a3",
        video_watched: true,
      }),
    );
    // 離れるときも、見終わったこととして保存する。
    unmount();
    expect(commands(calls, "save_video_position").at(-1)).toEqual({
      lesson_id: "ch04-02",
      last_sentence_id: "a3",
      video_watched: true,
    });
  });
});

describe("設定の保存", () => {
  it("字幕と再生速度を変えると、update_settings に設定の全体を送る", async () => {
    const { calls } = setup("ch04-02");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "字幕" }));
    await waitFor(() =>
      expect(commands(calls, "update_settings")).toHaveLength(1),
    );
    expect(commands(calls, "update_settings")[0]).toEqual({
      settings: {
        ...baseSettings,
        video: { ...baseSettings.video, captions: false },
      },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "再生速度" }), {
      target: { value: "2" },
    });
    await waitFor(() =>
      expect(commands(calls, "update_settings")).toHaveLength(2),
    );
    expect(commands(calls, "update_settings")[1]).toEqual({
      settings: {
        ...baseSettings,
        video: { ...baseSettings.video, captions: false, playback_rate: 2 },
      },
    });
  });
});
