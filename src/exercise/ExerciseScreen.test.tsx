import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { useEffect, useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Chapter, LessonKind } from "../commands/content";
import type { SubmitResult } from "../commands/run";
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
  // 開いたときに完了済みか。
  completed?: boolean;
  // get_passed_code の戻り値。無いときは not_found で拒否する。
  passedCode?: {
    session_id: string;
    submitted_at: string;
    files: { [path: string]: string };
  };
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
          progress:
            options.completed === true
              ? {
                  status: "completed",
                  first_completed_at: "2026-10-28T14:32:00+09:00",
                  last_completed_at: "2026-10-28T14:32:00+09:00",
                  completion_count: 1,
                  last_opened_at: "2026-11-24T10:00:00+09:00",
                  last_sentence_id: null,
                  video_watched: true,
                  last_passed_session_id: "s0",
                }
              : null,
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
      case "end_session":
        return null;
      case "get_passed_code":
        if (options.passedCode === undefined) {
          throw {
            code: "not_found",
            message: "前回の正解コードが見つかりません。",
            detail: null,
          };
        }
        return options.passedCode;
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

describe("判定結果と正解", () => {
  const requestId = (calls: Calls, name: string) =>
    (commands(calls, name).at(-1) as { request_id: string }).request_id;

  async function press(name: string) {
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name }));
    });
  }

  // 「提出」を押し、判定結果を run_finished で届ける。
  async function submit(calls: Calls, result: SubmitResult) {
    await press("提出");
    await act(async () => {
      await emit("run_finished", {
        request_id: requestId(calls, "submit_code"),
        kind: "submit",
        finished_at: "2026-11-24T10:00:00+09:00",
        error: null,
        run: null,
        submit: result,
        progress: null,
      });
    });
  }

  const WRONG: SubmitResult = {
    status: "wrong",
    passed: 1,
    total: 3,
    cases: [
      { name: "基本の名前", status: "pass", detail: null },
      {
        name: "前後に空白がある名前",
        status: "fail",
        detail: {
          input: "  ferris  \n",
          expected: "ferris_the_crab\n",
          actual: "  ferris_the_crab\n",
        },
      },
      { name: "長い名前", status: "timeout", detail: "時間切れ" },
    ],
  };
  const PASSED: SubmitResult = {
    status: "passed",
    passed: 3,
    total: 3,
    cases: [],
  };

  const card = () => screen.queryByRole("region", { name: "判定結果" });
  const passDialog = () => screen.getByRole("dialog", { name: "正解" });
  const solvedBar = () => {
    const bar = document.querySelector<HTMLElement>(".pg-solved");
    if (bar === null) {
      throw new Error("帯が無い");
    }
    return bar;
  };

  it("実行した後に提出して不正解になっても、実行の出力はコンソールに残り、問題文の上に不正解のカードを畳んで出す", async () => {
    const { calls, container } = setup("ch04-02");
    await editor();
    await press("実行");
    const runId = requestId(calls, "run_code");
    await act(async () => {
      await emit("run_output", {
        request_id: runId,
        chunks: [{ stream: "stdout", text: "実行の出力\n" }],
      });
      await emit("run_finished", {
        request_id: runId,
        kind: "run",
        finished_at: "2026-11-24T10:00:00+09:00",
        error: null,
        run: { mode: "main", status: "success", exit_code: 0 },
        submit: null,
        progress: null,
      });
    });
    expect(card()).toBeNull();
    await submit(calls, WRONG);

    expect(screen.getByTestId("console-output")).toHaveTextContent(
      "実行の出力",
    );
    const judge = card();
    expect(judge).not.toBeNull();
    expect(judge).toHaveTextContent("不正解");
    expect(judge).toHaveTextContent(
      /3 件中 2 件のテストが失敗しました ・ \d\d:\d\d に提出/,
    );
    // 問題文の上に置く。
    const side = container.querySelector(".exercise-side");
    expect(side?.firstElementChild).toBe(judge);
    // 詳細は畳んである。
    const toggle = screen.getByRole("button", { name: "詳細を見る" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(judge).not.toHaveTextContent("前後に空白がある名前");

    fireEvent.click(toggle);
    expect(
      screen.getByRole("button", { name: "詳細を閉じる" }),
    ).toHaveAttribute("aria-expanded", "true");
    const items = screen
      .getByRole("list", { name: "テストケース" })
      .querySelectorAll("li");
    expect([...items].map((li) => [li.textContent, li.className])).toEqual([
      ["基本の名前", "ok"],
      ["前後に空白がある名前", "ng"],
      ["長い名前", "ng"],
    ]);
    // 最初に失敗したテストケースの入力・期待する出力・実際の出力。行頭と行末の空白は ␣ で見せる。
    const detail = judge?.querySelector(".pg-case");
    expect(detail).toHaveTextContent("前後に空白がある名前");
    expect(detail).toHaveTextContent("入力␣␣ferris␣␣");
    expect(detail).toHaveTextContent("期待する出力ferris_the_crab");
    expect(detail).toHaveTextContent("実際の出力␣␣ferris_the_crab");
    expect(detail).not.toHaveTextContent("時間切れ");
  });

  it("test 方式の失敗は、最初に失敗したテストケースの panic メッセージを出す", async () => {
    const { calls } = setup("ch04-02");
    await editor();
    await submit(calls, {
      status: "wrong",
      passed: 0,
      total: 2,
      cases: [
        {
          name: "add_suffix_trims",
          status: "fail",
          detail: "assertion `left == right` failed\n  left: 1\n right: 2",
        },
        { name: "add_suffix_keeps", status: "runtime_error", detail: "panic" },
      ],
    });
    fireEvent.click(screen.getByRole("button", { name: "詳細を見る" }));
    const detail = card()?.querySelector(".pg-case");
    expect(detail).toHaveTextContent("add_suffix_trims");
    expect(detail).toHaveTextContent("assertion `left == right` failed");
    expect(card()).toHaveTextContent("add_suffix_keeps");
  });

  it("ビルドに失敗したときは、テストを実行していないことと、エラーがコンソールにあることを示す", async () => {
    const { calls } = setup("ch04-02");
    await editor();
    await submit(calls, {
      status: "compile_error",
      passed: 0,
      total: 0,
      cases: [],
    });
    expect(card()).toHaveTextContent("ビルドに失敗しました");
    expect(card()).toHaveTextContent(
      "テストは実行していません。エラーはコンソールに表示しています。",
    );
    expect(screen.queryByRole("button", { name: "詳細を見る" })).toBeNull();
  });

  it.each(["crates_unavailable", "stopped"] as const)(
    "%s で終わった提出では、前のカードをそのまま残す",
    async (status) => {
      const { calls } = setup("ch04-02");
      await editor();
      await submit(calls, WRONG);
      await submit(calls, { status, passed: 0, total: 0, cases: [] });
      expect(card()).toHaveTextContent("3 件中 2 件のテストが失敗しました");
    },
  );

  it("前のカードが無いときに crates_unavailable で終わっても、カードを出さない", async () => {
    const { calls } = setup("ch04-02");
    await editor();
    await submit(calls, {
      status: "crates_unavailable",
      passed: 0,
      total: 0,
      cases: [],
    });
    expect(card()).toBeNull();
  });

  it("次に提出すると、最新の結果に置き換え、詳細を畳んだ状態に戻す", async () => {
    const { calls } = setup("ch04-02");
    await editor();
    await submit(calls, WRONG);
    fireEvent.click(screen.getByRole("button", { name: "詳細を見る" }));
    await submit(calls, { ...WRONG, passed: 2 });
    expect(card()).toHaveTextContent("3 件中 1 件のテストが失敗しました");
    expect(screen.getByRole("button", { name: "詳細を見る" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("正解すると、カードを消して正解のモーダルを開き、解説・問題文・提出したコードを切り替えられる", async () => {
    const { calls } = setup("ch04-02");
    fireEvent.change(await editor(), {
      target: { value: "fn main() { /* 正解 */ }" },
    });
    await submit(calls, WRONG);
    await submit(calls, PASSED);
    expect(card()).toBeNull();

    const dialog = screen.getByRole("dialog", { name: "正解" });
    expect(dialog).toHaveTextContent(
      "3 件のテストがすべて通りました。4-2 所有権のムーブ を完了しました。",
    );
    expect(dialog).toHaveTextContent("4所有権");
    // 解説（explanation_md を Markdown で表示）。
    expect(screen.getByRole("tab", { name: "解説" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "考え方" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("tab", { name: "問題文" }));
    expect(screen.getByRole("tabpanel")).toHaveTextContent(
      "名前を読み込んでください。",
    );
    fireEvent.click(screen.getByRole("tab", { name: "提出したコード" }));
    expect(screen.getByRole("tabpanel")).toHaveTextContent("src/main.rs");
    expect(screen.getByRole("tabpanel")).toHaveTextContent(
      "fn main() { /* 正解 */ }",
    );
    // 質疑応答は M4 で作るので、準備中の表示にする。
    expect(
      screen.getByRole("complementary", { name: "解説について質問" }),
    ).toHaveTextContent("準備中");
    // 次のレッスン（学習順で後の、準備中でないレッスン）の番号と名前を添える。
    expect(
      within(dialog).getByRole("button", { name: /次のレッスンへ/ }),
    ).toHaveTextContent("4-3 第4章の復習");
    expect(editorProps.current?.readOnly).toBe(true);
  });

  it("「閉じてコードを見る」で、読み取り専用のエディタと帯を出し、「解説を開く」でモーダルを開き直す", async () => {
    const { calls } = setup("ch04-02", { layout: "multi" });
    await editor();
    await submit(calls, PASSED);
    // 下の「閉じてコードを見る」と、× の「閉じてコードを見る」がある。
    const closers = within(passDialog()).getAllByRole("button", {
      name: "閉じてコードを見る",
    });
    expect(closers).toHaveLength(2);
    fireEvent.click(closers[1]);
    expect(screen.queryByRole("dialog", { name: "正解" })).toBeNull();
    expect(solvedBar()).toHaveTextContent("正解しました");
    expect(screen.getByText("読み取り専用")).toBeVisible();
    expect(editorProps.current?.readOnly).toBe(true);
    // 解説表示では、下書きを書き換える操作をできなくする。
    expect(
      screen.getByRole("button", { name: "初期コードに戻す" }),
    ).toBeDisabled();
    expect(screen.queryByRole("button", { name: "ファイルを追加" })).toBeNull();
    expect(screen.getByRole("button", { name: "提出" })).toBeDisabled();

    fireEvent.click(
      within(solvedBar()).getByRole("button", { name: "解説を開く" }),
    );
    expect(screen.getByRole("dialog", { name: "正解" })).toBeVisible();
    // Esc でも閉じる。
    fireEvent.keyDown(screen.getByRole("dialog", { name: "正解" }), {
      key: "Escape",
    });
    expect(screen.queryByRole("dialog", { name: "正解" })).toBeNull();
  });

  it("次のレッスンが無いレッスンでは、「次のレッスンへ」を出さない", async () => {
    const { calls } = setup("ch04-03", { kind: "chapter_review" });
    await editor();
    await submit(calls, PASSED);
    expect(screen.queryByRole("button", { name: /次のレッスンへ/ })).toBeNull();
    expect(
      within(passDialog()).getByRole("button", { name: "レッスン一覧へ" }),
    ).toBeVisible();
    expect(
      within(solvedBar()).getByRole("button", { name: "レッスン一覧へ" }),
    ).toBeVisible();
  });

  describe("end_session", () => {
    it.each([
      ["モーダルの「レッスン一覧へ」", { name: "lessons" }],
      [
        "モーダルの「次のレッスンへ」",
        { name: "exercise", lessonId: "ch04-03" },
      ],
      ["帯の「レッスン一覧へ」", { name: "lessons" }],
      ["帯の「次のレッスンへ」", { name: "exercise", lessonId: "ch04-03" }],
      ["左上のリンク", { name: "lessons" }],
      ["「動画を見直す」", { name: "video", lessonId: "ch04-02" }],
    ] as const)(
      "解説表示で%sを押すと、end_session で試行セッションを終えてから移る",
      async (how, route) => {
        const { calls, navigate, unmount } = setup("ch04-02");
        await editor();
        await submit(calls, PASSED);
        const place = how.startsWith("帯") ? solvedBar() : passDialog();
        await act(async () => {
          if (how === "左上のリンク") {
            fireEvent.click(screen.getByRole("link", { name: "レッスン一覧" }));
          } else if (how === "「動画を見直す」") {
            fireEvent.click(
              screen.getByRole("button", { name: "動画を見直す" }),
            );
          } else if (how.includes("次のレッスンへ")) {
            fireEvent.click(
              within(place).getByRole("button", { name: /次のレッスンへ/ }),
            );
          } else {
            fireEvent.click(
              within(place).getByRole("button", { name: "レッスン一覧へ" }),
            );
          }
        });
        await flush();
        expect(commands(calls, "end_session")).toEqual([
          { lesson_id: "ch04-02" },
        ]);
        expect(navigate).toHaveBeenCalledTimes(1);
        expect(navigate).toHaveBeenCalledWith(route);
        // 移った後に画面が消えても、もう一度は呼ばない。
        unmount();
        await flush();
        expect(commands(calls, "end_session")).toHaveLength(1);
      },
    );

    it("ヘッダーのメニューなど、画面の外から移ったとき（アンマウント）も、解説表示なら試行セッションを終える", async () => {
      const { calls, unmount } = setup("ch04-02");
      fireEvent.change(await editor(), { target: { value: "fn main() {}" } });
      // 提出中に編集すると、保存を待つ状態になる。正解したら保存をやめ、終えた後に下書きを作り直さない。
      await press("提出");
      // 提出のファイルを読むのは、提出のコマンドを送る直前である。その後に編集するように、送られたのを待つ。
      await waitFor(() =>
        expect(commands(calls, "submit_code")).toHaveLength(1),
      );
      fireEvent.change(await editor(), {
        target: { value: "// 提出中の編集" },
      });
      await act(async () => {
        await emit("run_finished", {
          request_id: requestId(calls, "submit_code"),
          kind: "submit",
          finished_at: "2026-11-24T10:00:00+09:00",
          error: null,
          run: null,
          submit: PASSED,
          progress: null,
        });
      });
      // 読み取り専用のエディタには、提出したコードを見せる。
      expect(await editor()).toHaveValue("fn main() {}");
      expect(editorProps.current?.readOnly).toBe(true);
      unmount();
      await flush();
      expect(commands(calls, "end_session")).toEqual([
        { lesson_id: "ch04-02" },
      ]);
      const names = calls.map((c) => c.command);
      expect(names.slice(names.indexOf("end_session"))).not.toContain(
        "save_draft",
      );
    });

    it("解答中に画面を離れても、end_session は呼ばない", async () => {
      const { calls, navigate, unmount } = setup("ch04-02");
      await editor();
      await submit(calls, WRONG);
      fireEvent.click(screen.getByRole("link", { name: "レッスン一覧" }));
      expect(navigate).toHaveBeenCalledWith({ name: "lessons" });
      unmount();
      await flush();
      expect(commands(calls, "end_session")).toEqual([]);
    });
  });
});

describe("前回の正解コードを見る", () => {
  it("完了済みのレッスンでは、押すと get_passed_code の結果を読み取り専用のモーダルで見せる", async () => {
    const submittedAt = "2026-10-28T14:32:00+09:00";
    const { calls } = setup("ch04-02", {
      completed: true,
      passedCode: {
        session_id: "s0",
        submitted_at: submittedAt,
        files: { "src/main.rs": "// 前回の正解\n" },
      },
    });
    expect(await editor()).toHaveValue(STARTER);
    fireEvent.click(
      screen.getByRole("button", { name: "前回の正解コードを見る" }),
    );
    const dialog = screen.getByRole("dialog", { name: "前回の正解コード" });
    expect(
      await screen.findByRole("textbox", {
        name: "前回の正解コード src/main.rs",
      }),
    ).toHaveValue("// 前回の正解\n");
    expect(editorProps.current?.readOnly).toBe(true);
    const at = new Date(submittedAt);
    expect(dialog).toHaveTextContent(
      `${at.getMonth() + 1} 月 ${at.getDate()} 日 ${String(at.getHours()).padStart(2, "0")}:${String(at.getMinutes()).padStart(2, "0")} に正解 ・ 読み取り専用`,
    );
    expect(commands(calls, "get_passed_code")).toEqual([
      { lesson_id: "ch04-02" },
    ]);
    // 足元の「閉じる」で閉じる。編集中のコードには影響しない。
    fireEvent.click(
      within(dialog).getAllByRole("button", { name: "閉じる" })[1],
    );
    expect(
      screen.queryByRole("dialog", { name: "前回の正解コード" }),
    ).toBeNull();
    expect(await editor()).toHaveValue(STARTER);
  });

  it("前回の正解コードが見つからないときは、読み込めなかったことを示す", async () => {
    setup("ch04-02", { completed: true });
    await editor();
    fireEvent.click(
      screen.getByRole("button", { name: "前回の正解コードを見る" }),
    );
    expect(
      await screen.findByText("前回の正解コードを読み込めませんでした。"),
    ).toBeVisible();
  });

  it("完了していないレッスンでは出さない", async () => {
    setup("ch04-02");
    await editor();
    expect(
      screen.queryByRole("button", { name: "前回の正解コードを見る" }),
    ).toBeNull();
  });
});
