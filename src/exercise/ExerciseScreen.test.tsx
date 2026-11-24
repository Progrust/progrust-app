import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { useEffect, useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Chapter, LessonKind } from "../commands/content";
import type { CodeEditorProps } from "../editor";
import { NavigationContext } from "../router";
import type { Route } from "../router";
import { ExerciseScreen } from "./ExerciseScreen";

// jsdom では Monaco を描画できないので、エディタを textarea に置き換える。
// 実物（@monaco-editor/react）と同じく、内容を画面から差し替えたときにも onChange を呼ぶ。
const editorProps = vi.hoisted(() => ({
  current: null as CodeEditorProps | null,
}));
vi.mock("../editor", () => ({
  CodeEditor: function FakeEditor(props: CodeEditorProps) {
    editorProps.current = props;
    const { value, onChange, ariaLabel } = props;
    const shown = useRef(value);
    useEffect(() => {
      if (shown.current !== value) {
        shown.current = value;
        onChange?.(value);
      }
    });
    return (
      <textarea
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => {
          shown.current = event.target.value;
          onChange?.(event.target.value);
        }}
      />
    );
  },
}));

const STARTER = 'fn main() {\n    println!("start");\n}\n';
const PROBLEM = [
  "## 問題",
  "",
  "名前を読み込んでください。",
  "",
  "```",
  "名前",
  "```",
  "",
  '文中の <b class="raw">太字</b> のタグ',
  "",
  "<script>alert(1)</script>",
  "",
  "[リンク](https://example.com)",
].join("\n");

const chapters: Chapter[] = [
  { id: "ch01", title: "はじめに", description: "", lessons: [] },
  { id: "ch02", title: "変数", description: "", lessons: [] },
  { id: "ch03", title: "型", description: "", lessons: [] },
  {
    id: "ch04",
    title: "所有権",
    description: "",
    lessons: [
      summary("ch04-01", "video_only", "メモリの仕組み"),
      summary("ch04-02", "lesson", "所有権のムーブ"),
      summary("ch04-03", "chapter_review", "第4章の復習"),
    ],
  },
];

function summary(id: string, kind: LessonKind, title: string) {
  return {
    id,
    kind,
    title,
    summary: null,
    estimated_minutes: null,
    in_preparation: false,
  };
}

type Calls = { command: string; args: unknown }[];

type Options = {
  kind?: LessonKind;
  // レッスン ID から下書きのファイルへの対応。無いレッスンは、下書きが無い。
  drafts?: { [lessonId: string]: { [path: string]: string } };
  starter?: { [path: string]: string };
  failLesson?: boolean;
  layout?: "single" | "multi";
  readonlyFiles?: string[];
};

function mockCommands(calls: Calls, options: Options = {}) {
  let lessonFailures = options.failLesson === true ? 1 : 0;
  const handle: Parameters<typeof mockIPC>[0] = (command, args) => {
    calls.push({ command, args });
    const lessonId = (args as { lesson_id?: string }).lesson_id ?? "";
    const title =
      chapters[3].lessons.find((l) => l.id === lessonId)?.title ?? "";
    const starter = options.starter ?? { "src/main.rs": STARTER };
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
            kind: options.kind ?? "lesson",
            title,
            primary_elements: [],
            secondary_elements: [],
            ...(options.layout === undefined
              ? {}
              : {
                  exercise: {
                    layout: options.layout,
                    judge: { mode: "stdout", target: "src/main.rs" },
                    dependencies: [],
                    readonly_files: options.readonlyFiles ?? [],
                    case_labels: {},
                  },
                }),
          },
          script: null,
          voice_dir: null,
          exercise: {
            problem_md: PROBLEM,
            explanation_md: "## 考え方\n",
            starter_files: starter,
          },
        };
      case "open_lesson": {
        const files = options.drafts?.[lessonId];
        return {
          session_id: "s1",
          progress: null,
          generated: null,
          draft:
            files === undefined
              ? null
              : {
                  schema_version: 1,
                  lesson_id: lessonId,
                  updated_at: "2026-11-01T00:00:00+09:00",
                  files,
                },
        };
      }
      case "get_settings":
        return {
          schema_version: 1,
          theme: "system",
          editor_font_size: 18,
          video: { volume: 1, muted: false, playback_rate: 1, captions: false },
          models: { chat: "sonnet", background: "opus" },
          tool_paths: { cargo: null, claude: null },
          first_launch_completed: true,
        };
      case "save_draft":
        return null;
      case "reset_draft":
        return {
          schema_version: 1,
          lesson_id: lessonId,
          updated_at: "2026-11-01T00:00:00+09:00",
          files: starter,
        };
      default:
        return undefined;
    }
  };
  mockIPC(handle, { shouldMockEvents: true });
}

function setup(lessonId: string, options: Options = {}) {
  const calls: Calls = [];
  mockCommands(calls, options);
  const navigate = vi.fn<(route: Route) => void>();
  const ui = (id: string) => (
    <NavigationContext
      value={{ route: { name: "exercise", lessonId: id }, navigate }}
    >
      {/* App と同じく、レッスンごとに作り直す。 */}
      <ExerciseScreen key={id} lessonId={id} />
    </NavigationContext>
  );
  const view = render(ui(lessonId));
  return {
    calls,
    navigate,
    ...view,
    showLesson: (id: string) => view.rerender(ui(id)),
  };
}

function commands(calls: Calls, name: string) {
  return calls.filter((c) => c.command === name).map((c) => c.args);
}

async function editor() {
  return screen.findByRole("textbox", { name: "エディタ src/main.rs" });
}

// 保存の命令は、コマンドを 1 つずつ送る列に入るので、Promise を進めてから数える。
async function flush() {
  await act(async () => {});
}

afterEach(() => {
  // 画面を離れるときの保存・stop_run・受け取りの解除が、置き換えたコマンドに届くように、先にアンマウントする。
  cleanup();
  clearMocks();
  vi.useRealTimers();
  editorProps.current = null;
});

describe("開いたとき", () => {
  it("get_lesson と open_lesson を 1 回ずつ呼び、下書きが無ければ初期コードを表示する", async () => {
    const { calls } = setup("ch04-02");
    expect(await editor()).toHaveValue(STARTER);
    expect(commands(calls, "get_lesson")).toEqual([{ lesson_id: "ch04-02" }]);
    expect(commands(calls, "open_lesson")).toEqual([{ lesson_id: "ch04-02" }]);
    // 文字サイズは設定の editor_font_size。
    expect(editorProps.current?.fontSize).toBe(18);
    expect(screen.getByRole("tab", { name: "src/main.rs" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("下書きがあれば、下書きのコードを表示する", async () => {
    setup("ch04-02", {
      drafts: { "ch04-02": { "src/main.rs": "// 書きかけ\n" } },
    });
    expect(await editor()).toHaveValue("// 書きかけ\n");
  });

  it("上の帯に、レッスン一覧へのリンク・章のチップ・レッスンの番号と名前・「動画を見直す」を出し、章カラーを付ける", async () => {
    const { container } = setup("ch04-02");
    await editor();
    expect(screen.getByRole("link", { name: "レッスン一覧" })).toBeVisible();
    expect(container.querySelector(".pg-chip")).toHaveTextContent("4所有権");
    expect(container.querySelector(".pg-context-title")).toHaveTextContent(
      "4-2 所有権のムーブ",
    );
    expect(screen.getByRole("button", { name: "動画を見直す" })).toBeVisible();
    expect(container.querySelector("main")).toHaveAttribute(
      "data-chapter",
      "4",
    );
  });

  it("問題文を Markdown として表示し、生の HTML とリンクの行き先は表示しない", async () => {
    const { container } = setup("ch04-02");
    await editor();
    const problem = screen.getByRole("region", { name: "問題文" });
    expect(
      screen.getByRole("heading", { level: 2, name: "問題" }),
    ).toBeVisible();
    expect(problem).toHaveTextContent("名前を読み込んでください。");
    expect(problem.querySelector("pre.pg-code")).toHaveTextContent("名前");
    expect(problem.querySelector(".raw")).toBeNull();
    expect(problem.querySelector("script")).toBeNull();
    // 文中のタグは捨て、中の文字だけを表示する。HTML のブロックは、まるごと捨てる。
    expect(problem).toHaveTextContent("文中の 太字 のタグ");
    expect(problem).not.toHaveTextContent("<b");
    expect(problem).not.toHaveTextContent("alert");
    expect(problem).toHaveTextContent("リンク");
    expect(problem.querySelector("a")).toBeNull();
    // コンソールは、出力の無い状態で置く。
    expect(
      screen.getByRole("region", { name: "コンソール" }),
    ).toBeInTheDocument();
    expect(screen.getByTestId("console-output")).toHaveTextContent("");
    expect(screen.getByRole("textbox", { name: "標準入力" })).toBeDisabled();
    expect(container.querySelectorAll('[role="separator"]')).toHaveLength(2);
  });

  it("章末復習レッスンでは「動画を見直す」を出さない", async () => {
    setup("ch04-03", { kind: "chapter_review" });
    await editor();
    expect(screen.queryByRole("button", { name: "動画を見直す" })).toBeNull();
    expect(screen.getByRole("link", { name: "レッスン一覧" })).toBeVisible();
  });

  it("読み込めなかったら「もう一度読み込む」で読み込み直し、open_lesson は呼び直さない", async () => {
    const { calls } = setup("ch04-02", { failLesson: true });
    await screen.findByText("演習を読み込めませんでした。");
    fireEvent.click(screen.getByRole("button", { name: "もう一度読み込む" }));
    expect(await editor()).toHaveValue(STARTER);
    expect(commands(calls, "get_lesson")).toHaveLength(2);
    expect(commands(calls, "open_lesson")).toHaveLength(1);
  });

  it("別のレッスンの演習画面に移ると、前のレッスンのコードを残さない", async () => {
    const { showLesson } = setup("ch04-02", {
      drafts: { "ch04-02": { "src/main.rs": "// 前のレッスン\n" } },
    });
    expect(await editor()).toHaveValue("// 前のレッスン\n");
    showLesson("ch04-03");
    // 読み込み直すまで、前のコードを表示しない。
    expect(
      screen.queryByRole("textbox", { name: "エディタ src/main.rs" }),
    ).toBeNull();
    expect(await screen.findByText("4-3 第4章の復習")).toBeVisible();
    expect(await editor()).toHaveValue(STARTER);
  });
});

describe("移る", () => {
  it("左上のリンクでレッスン一覧に、「動画を見直す」でそのレッスンの動画画面に移る", async () => {
    const { navigate } = setup("ch04-02");
    await editor();
    fireEvent.click(screen.getByRole("link", { name: "レッスン一覧" }));
    expect(navigate).toHaveBeenLastCalledWith({ name: "lessons" });
    fireEvent.click(screen.getByRole("button", { name: "動画を見直す" }));
    expect(navigate).toHaveBeenLastCalledWith({
      name: "video",
      lessonId: "ch04-02",
    });
  });
});

describe("下書きの保存", () => {
  it("最後の編集から 1 秒後に、エディタ上の全ファイルを save_draft で保存する", async () => {
    const { calls } = setup("ch04-02", {
      drafts: {
        "ch04-02": { "src/main.rs": "a", "src/geo.rs": "pub fn f() {}" },
      },
    });
    const textbox = await editor();
    vi.useFakeTimers();
    fireEvent.change(textbox, { target: { value: "ab" } });
    act(() => vi.advanceTimersByTime(500));
    fireEvent.change(textbox, { target: { value: "abc" } });
    act(() => vi.advanceTimersByTime(999));
    await flush();
    expect(commands(calls, "save_draft")).toEqual([]);
    act(() => vi.advanceTimersByTime(1));
    await flush();
    expect(commands(calls, "save_draft")).toEqual([
      {
        lesson_id: "ch04-02",
        files: { "src/main.rs": "abc", "src/geo.rs": "pub fn f() {}" },
      },
    ]);
  });

  it("画面を離れるとき、保存していない編集があれば保存する", async () => {
    const { calls, unmount } = setup("ch04-02");
    fireEvent.change(await editor(), { target: { value: "fn main() {}" } });
    unmount();
    await flush();
    expect(commands(calls, "save_draft")).toEqual([
      { lesson_id: "ch04-02", files: { "src/main.rs": "fn main() {}" } },
    ]);
  });

  it("編集していなければ、画面を離れても保存しない", async () => {
    const { calls, unmount } = setup("ch04-02");
    await editor();
    unmount();
    await flush();
    expect(commands(calls, "save_draft")).toEqual([]);
  });
});

describe("初期コードに戻す", () => {
  it("確認のダイアログで「キャンセル」を押すと、何もしない", async () => {
    const { calls } = setup("ch04-02", {
      drafts: { "ch04-02": { "src/main.rs": "x" } },
    });
    await editor();
    fireEvent.click(screen.getByRole("button", { name: "初期コードに戻す" }));
    const dialog = screen.getByRole("alertdialog", {
      name: "初期コードに戻しますか？",
    });
    expect(screen.getByRole("button", { name: "キャンセル" })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "キャンセル" }));
    expect(dialog).not.toBeInTheDocument();
    expect(commands(calls, "reset_draft")).toEqual([]);
    expect(await editor()).toHaveValue("x");
  });

  it("Esc で閉じる", async () => {
    setup("ch04-02");
    await editor();
    fireEvent.click(screen.getByRole("button", { name: "初期コードに戻す" }));
    fireEvent.keyDown(screen.getByRole("button", { name: "キャンセル" }), {
      key: "Escape",
    });
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("確認の後に reset_draft を呼び、戻した下書きを表示する。待っていた保存はしない", async () => {
    const { calls, unmount } = setup("ch04-02", {
      drafts: { "ch04-02": { "src/main.rs": "x" } },
    });
    const textbox = await editor();
    vi.useFakeTimers();
    fireEvent.change(textbox, { target: { value: "xy" } });
    fireEvent.click(screen.getByRole("button", { name: "初期コードに戻す" }));
    const dialog = screen.getByRole("alertdialog");
    fireEvent.click(
      screen.getAllByRole("button", { name: "初期コードに戻す" }).at(-1)!,
    );
    await flush();
    expect(commands(calls, "reset_draft")).toEqual([{ lesson_id: "ch04-02" }]);
    expect(dialog).not.toBeInTheDocument();
    expect(textbox).toHaveValue(STARTER);
    // 戻した後に、1 秒後の保存と、離れるときの保存で、古いコードを書かない。
    act(() => vi.advanceTimersByTime(2000));
    unmount();
    await flush();
    expect(commands(calls, "save_draft")).toEqual([]);
  });
});

describe("multi レイアウトのファイル操作", () => {
  const MULTI = {
    "src/main.rs": "fn main() {}\n",
    "src/geo.rs": "// geo\n",
    "tests/it.rs": "// it\n",
  };
  const multi = (extra: Options = {}) =>
    setup("ch04-02", {
      layout: "multi",
      starter: MULTI,
      readonlyFiles: ["tests/it.rs"],
      ...extra,
    });

  async function addFileNamed(path: string) {
    fireEvent.click(
      await screen.findByRole("button", { name: "ファイルを追加" }),
    );
    fireEvent.change(
      screen.getByRole("textbox", { name: "新しいファイルのパス" }),
      {
        target: { value: path },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "追加" }));
  }

  it("single では、追加の + を出さない", async () => {
    setup("ch04-02", { layout: "single" });
    await editor();
    expect(screen.queryByRole("button", { name: "ファイルを追加" })).toBeNull();
    expect(screen.getAllByRole("tab")).toHaveLength(1);
  });

  it("全ファイルのタブを出し、切り替えるとエディタの内容が変わる", async () => {
    multi();
    await editor();
    expect(screen.getAllByRole("tab").map((t) => t.textContent)).toEqual([
      "src/main.rs",
      "src/geo.rs",
      "tests/it.rs",
    ]);
    fireEvent.click(screen.getByRole("tab", { name: "src/geo.rs" }));
    expect(
      await screen.findByRole("textbox", { name: "エディタ src/geo.rs" }),
    ).toHaveValue("// geo\n");
    expect(editorProps.current?.readOnly).toBe(false);
  });

  it("readonly_files のファイルは読み取り専用で、消す・名前を変えるボタンが無い", async () => {
    multi();
    await editor();
    fireEvent.click(screen.getByRole("tab", { name: "tests/it.rs" }));
    expect(editorProps.current?.readOnly).toBe(true);
    expect(screen.queryByRole("button", { name: "ファイルを削除" })).toBeNull();
    expect(
      screen.queryByRole("button", { name: "ファイルの名前を変える" }),
    ).toBeNull();
  });

  it("src/main.rs は、消せず、名前も変えられない", async () => {
    multi();
    await editor();
    expect(screen.queryByRole("button", { name: "ファイルを削除" })).toBeNull();
    expect(
      screen.queryByRole("button", { name: "ファイルの名前を変える" }),
    ).toBeNull();
  });

  it("src/ と tests/ の外や .rs でないパス、同じ名前のファイルは作れない", async () => {
    const { calls } = multi();
    await editor();
    for (const bad of [
      "main.rs",
      "src/a.txt",
      "docs/a.rs",
      "src/../a.rs",
      "/src/a.rs",
      "src/geo.rs",
    ]) {
      await addFileNamed(bad);
      expect(screen.getByRole("alert")).toBeVisible();
      expect(screen.getAllByRole("tab")).toHaveLength(3);
      fireEvent.click(screen.getByRole("button", { name: "やめる" }));
    }
    expect(commands(calls, "save_draft")).toEqual([]);
  });

  it("追加したファイルを下書きに保存し、開き直すと戻る", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { calls, unmount } = multi();
    await editor();
    await addFileNamed("src/util.rs");
    expect(
      await screen.findByRole("textbox", { name: "エディタ src/util.rs" }),
    ).toHaveValue("");
    await act(async () => {
      vi.advanceTimersByTime(1000);
    });
    await flush();
    const saved = commands(calls, "save_draft") as {
      files: Record<string, string>;
    }[];
    expect(saved).toHaveLength(1);
    expect(Object.keys(saved[0].files)).toEqual([
      "src/main.rs",
      "src/geo.rs",
      "tests/it.rs",
      "src/util.rs",
    ]);
    unmount();
    clearMocks();
    setup("ch04-02", {
      layout: "multi",
      starter: MULTI,
      drafts: { "ch04-02": saved[0].files },
    });
    await editor();
    expect(screen.getByRole("tab", { name: "src/util.rs" })).toBeVisible();
  });

  it("名前を変える・削除すると、タブと下書きに反映する", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { calls } = multi();
    await editor();
    fireEvent.click(screen.getByRole("tab", { name: "src/geo.rs" }));
    fireEvent.click(
      screen.getByRole("button", { name: "ファイルの名前を変える" }),
    );
    fireEvent.change(screen.getByRole("textbox", { name: "新しいパス" }), {
      target: { value: "src/shape.rs" },
    });
    fireEvent.click(screen.getByRole("button", { name: "変更" }));
    expect(screen.getByRole("tab", { name: "src/shape.rs" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.queryByRole("tab", { name: "src/geo.rs" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "ファイルを削除" }));
    expect(screen.getAllByRole("tab")).toHaveLength(2);
    await act(async () => {
      vi.advanceTimersByTime(1000);
    });
    await flush();
    const saved = commands(calls, "save_draft") as {
      files: Record<string, string>;
    }[];
    expect(Object.keys(saved[saved.length - 1].files)).toEqual([
      "src/main.rs",
      "tests/it.rs",
    ]);
  });
});

describe("実行・提出", () => {
  const requestId = (calls: Calls, name: string) =>
    (commands(calls, name).at(-1) as { request_id: string }).request_id;

  it("「実行」で、エディタ上の全ファイルを run_code に渡す", async () => {
    const { calls } = setup("ch04-02");
    fireEvent.change(await editor(), { target: { value: "fn main() {}" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "実行" }));
    });
    expect(commands(calls, "run_code")).toEqual([
      {
        request_id: expect.any(String),
        lesson_id: "ch04-02",
        mode: "main",
        files: { "src/main.rs": "fn main() {}" },
      },
    ]);
  });

  it.each([
    ["実行", "run_code"],
    ["テスト", "run_code"],
    ["提出", "submit_code"],
  ])(
    "%sの途中で左上のリンクから画面を離れると、stop_run で止める",
    async (name, command) => {
      const { calls, navigate, unmount } = setup("ch04-02");
      await editor();
      await act(async () => {
        fireEvent.click(screen.getByRole("button", { name }));
      });
      const id = requestId(calls, command);
      fireEvent.click(screen.getByRole("link", { name: "レッスン一覧" }));
      expect(navigate).toHaveBeenLastCalledWith({ name: "lessons" });
      // 画面を移ると、演習画面はアンマウントされる。
      unmount();
      expect(commands(calls, "stop_run")).toEqual([{ request_id: id }]);
    },
  );

  it("提出で正解すると、エディタが読み取り専用になる", async () => {
    const { calls } = setup("ch04-02");
    await editor();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "提出" }));
    });
    expect(editorProps.current?.readOnly).toBe(false);
    await act(async () => {
      await emit("run_finished", {
        request_id: requestId(calls, "submit_code"),
        kind: "submit",
        finished_at: "2026-11-24T10:00:00+09:00",
        error: null,
        run: null,
        submit: { status: "passed", passed: 1, total: 1, cases: [] },
        progress: null,
      });
    });
    expect(editorProps.current?.readOnly).toBe(true);
  });
});
