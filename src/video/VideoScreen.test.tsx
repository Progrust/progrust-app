import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import {
  act,
  cleanup,
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
  {
    id: "ch01",
    title: "はじめに",
    description: "",
    language: "rust",
    number: 1,
    lessons: [],
  },
  {
    id: "ch02",
    title: "変数",
    description: "",
    language: "rust",
    number: 2,
    lessons: [],
  },
  {
    id: "ch03",
    title: "型",
    description: "",
    language: "rust",
    number: 3,
    lessons: [],
  },
  {
    id: "ch04",
    title: "所有権",
    description: "",
    language: "rust",
    number: 4,
    lessons: [
      lesson("ch04-01", "video_only", "メモリの仕組み"),
      lesson("ch04-02", "lesson", "所有権のムーブ"),
      lesson("ch04-03", "lesson", "借用"),
    ],
  },
  {
    id: "ch05",
    title: "参照と借用",
    description: "",
    language: "rust",
    number: 5,
    lessons: [
      lesson("ch05-01", "video_only", "参照の仕組み"),
      lesson("ch05-02", "chapter_review", "第5章の復習"),
    ],
  },
];

function lesson(
  id: string,
  kind: "lesson" | "video_only" | "chapter_review",
  title: string,
) {
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
    options.kind ??
    (lessonId === "ch04-01" || lessonId === "ch05-01"
      ? "video_only"
      : "lesson");
  const calls: Calls = [];
  let lessonFailures = options.failLesson === true ? 1 : 0;
  mockIPC(
    (command, args) => {
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
              // 章は、レッスンの ID の頭（ch05-01 なら ch05）。違う章を返すと、動画の表示が失敗する。
              chapter: lessonId.split("-")[0],
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
    },
    { shouldMockEvents: true },
  );
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
  // 部品を外してから（チャットの受け取りをやめてから）、モックを外す。
  cleanup();
  clearMocks();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

// 読み込みが済むまで待つ。「再生」ボタンは、読み込み中も押せない状態で出ているので、
// 出たことではなく、押せるようになったことを待つ。
async function ready() {
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "再生" })).toBeEnabled(),
  );
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
    await waitFor(() =>
      expect(container.querySelector(".pg-video-time b")).toHaveTextContent(
        "0:02",
      ),
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
    fireEvent.click(await screen.findByRole("button", { name: "演習へ" }));
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
    fireEvent.click(
      await screen.findByRole("button", { name: "次のレッスンへ" }),
    );
    expect(navigate).toHaveBeenCalledWith({
      name: "video",
      lessonId: "ch04-02",
    });
  });

  it("次のレッスンが章末復習レッスンなら、「次のレッスンへ」で演習画面に移る", async () => {
    const { navigate } = setup("ch05-01", {
      progress: { video_watched: true },
    });
    await ready();
    fireEvent.click(
      await screen.findByRole("button", { name: "次のレッスンへ" }),
    );
    expect(navigate).toHaveBeenCalledWith({
      name: "exercise",
      lessonId: "ch05-02",
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
    fireEvent.click(await screen.findByRole("button", { name: "演習へ" }));
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
    fireEvent.click(
      await screen.findByRole("button", { name: "次のレッスンへ" }),
    );
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

describe("質問のチャットパネル", () => {
  const input = () =>
    screen.getByRole("textbox", { name: "この動画について質問" });

  // 質問を入力して、Cmd+Enter で送信する。
  async function ask(text: string) {
    fireEvent.change(input(), { target: { value: text } });
    fireEvent.keyDown(input(), { key: "Enter", metaKey: true });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }

  it("開いたときは閉じていて、「質問する」で開き、閉じるボタンで閉じる", async () => {
    setup("ch04-02");
    await ready();
    expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    expect(screen.getByRole("complementary")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "質問する" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "質問パネルを閉じる" }));
    expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "質問する" })).toBeVisible();
  });

  it("一時停止中に質問でき、再生と一時停止は変わらない", async () => {
    const { calls } = setup("ch04-02", {
      progress: { last_sentence_id: "a2" },
    });
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    await ask("二つ目について教えて");
    expect(commands(calls, "send_chat")).toEqual([
      expect.objectContaining({
        lesson_id: "ch04-02",
        context: "video",
        sentence_id: "a2",
        question: "二つ目について教えて",
      }),
    ]);
    expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
    expect(HTMLMediaElement.prototype.pause).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "再生" })).toBeVisible();
  });

  it("再生中に質問しても、動画は止まらない", async () => {
    setup("ch04-02");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "再生" }));
    await screen.findByRole("button", { name: "一時停止" });
    const pauses = vi.mocked(HTMLMediaElement.prototype.pause).mock.calls
      .length;
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    await ask("いまの話は？");
    expect(screen.getByRole("button", { name: "一時停止" })).toBeVisible();
    expect(vi.mocked(HTMLMediaElement.prototype.pause).mock.calls).toHaveLength(
      pauses,
    );
  });

  it("再生位置を移した後の質問には、移した先の文の ID を送る", async () => {
    const { calls, container } = setup("ch04-02", {
      progress: { last_sentence_id: "a1" },
    });
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    await ask("最初の質問");
    fireEvent.click(screen.getByRole("button", { name: /5 ?秒進む/ }));
    await waitFor(() =>
      expect(container.querySelector(".pg-captions")).toHaveTextContent(
        "三つ目",
      ),
    );
    // 回答中は次の質問を送れないので、回答を終わらせる。
    const first = commands(calls, "send_chat")[0] as { request_id: string };
    await act(async () => {
      await emit("chat_finished", {
        request_id: first.request_id,
        status: "completed",
        error: null,
      });
    });
    await ask("移した先の質問");
    const sent = commands(calls, "send_chat");
    expect(sent).toHaveLength(2);
    expect(sent[0]).toMatchObject({ sentence_id: "a1" });
    expect(sent[1]).toMatchObject({ sentence_id: "a3" });
  });

  it("画面を離れて開き直すと、会話は消えている", async () => {
    const first = setup("ch04-02");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    await ask("消える質問");
    expect(screen.getByText("消える質問")).toBeVisible();
    first.unmount();
    setup("ch04-02");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    expect(screen.queryByText("消える質問")).not.toBeInTheDocument();
  });

  it("パネルを閉じても、会話は残る", async () => {
    setup("ch04-02");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    await ask("残る質問");
    fireEvent.click(screen.getByRole("button", { name: "質問パネルを閉じる" }));
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    expect(screen.getByText("残る質問")).toBeVisible();
  });

  it("チャットの入力欄では、Space で再生と一時停止を切り替えない", async () => {
    setup("ch04-02");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    fireEvent.keyDown(input(), { code: "Space", key: " " });
    expect(HTMLMediaElement.prototype.play).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { code: "Space", key: " " });
    await waitFor(() =>
      expect(HTMLMediaElement.prototype.play).toHaveBeenCalled(),
    );
  });

  it("動画のみのレッスンでも質問できる", async () => {
    const { calls } = setup("ch04-01");
    await ready();
    fireEvent.click(screen.getByRole("button", { name: "質問する" }));
    await ask("質問です");
    expect(commands(calls, "send_chat")).toHaveLength(1);
  });
});
