import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  RunFinished,
  RunOutputChunk,
  SubmitProgress,
} from "../commands/run";
import { ExerciseConsole } from "./ExerciseConsole";
import type { ExerciseState } from "./exerciseState";
import type { SubmitFinished } from "./judgeResult";
import type { ShortcutAction } from "./shortcuts";

type Call = { command: string; args: Record<string, unknown> };

type Options = {
  // コマンドの名前から、拒否するときの値への対応。
  reject?: { [command: string]: unknown };
  // run_code・submit_code が、呼ばれてすぐ（戻り値より先に）送るイベント。
  emitOnStart?: (requestId: string) => Promise<void>;
  // run_code・submit_code の戻りを、この Promise が解決するまで待たせる。
  hold?: Promise<void>;
  // 演習が使う許可クレート。
  dependencies?: string[];
  // get_environment_status が返す、外部クレートの準備の状態。
  crates?: "not_started" | "preparing" | "ready" | "failed";
};

const FILES = { "src/main.rs": "fn main() {}\n" };

function setup(options: Options = {}) {
  const calls: Call[] = [];
  mockIPC(
    async (command, args) => {
      const payload = (args ?? {}) as Record<string, unknown>;
      calls.push({ command, args: payload });
      if (command === "get_environment_status" && options.crates) {
        return {
          cargo: {
            status: "found",
            cargo_path: "/c/cargo",
            rustc_path: "/c/rustc",
            rustc_version: "1.100.0",
            required_rustc_version: "1.100.0",
          },
          claude: { status: "found", path: "/c/claude", version: "1.0.0" },
          clang: { status: "found", path: "/usr/bin/clang", version: "21.0.0" },
          crates: { status: options.crates, prepared_at: null, error: null },
        };
      }
      if (command === "run_code" || command === "submit_code") {
        await options.emitOnStart?.(payload.request_id as string);
        await options.hold;
      }
      if (options.reject?.[command] !== undefined) {
        throw options.reject[command];
      }
      return null;
    },
    { shouldMockEvents: true },
  );
  const draftSave = { cancel: vi.fn(), edited: vi.fn() };
  const states: ExerciseState[] = [];
  const submits: SubmitFinished[] = [];
  const shortcutRef: { current: ((action: ShortcutAction) => void) | null } = {
    current: null,
  };
  const view = render(
    <ExerciseConsole
      lessonId="ch04-04"
      dependencies={options.dependencies ?? []}
      getFiles={() => FILES}
      enqueue={(task) => task()}
      draftSave={draftSave}
      onStateChange={(state) => states.push(state)}
      onSubmitFinished={(finished) => submits.push(finished)}
      shortcutRef={shortcutRef}
    />,
  );
  return { calls, draftSave, states, submits, shortcutRef, ...view };
}

const button = (name: string) => screen.getByRole("button", { name });
const output = () => screen.getByTestId("console-output");

function commands(calls: Call[], name: string) {
  return calls.filter((c) => c.command === name).map((c) => c.args);
}

// 最後に呼んだ run_code・submit_code の request_id。
function lastRequestId(calls: Call[]): string {
  const started = calls.filter(
    (c) => c.command === "run_code" || c.command === "submit_code",
  );
  return started[started.length - 1].args.request_id as string;
}

async function press(name: string) {
  await act(async () => {
    fireEvent.click(button(name));
  });
}

async function send(event: string, payload: unknown) {
  await act(async () => {
    await emit(event, payload);
  });
}

const sendOutput = (requestId: string, chunks: RunOutputChunk[]) =>
  send("run_output", { request_id: requestId, chunks });

const sendProgress = (
  requestId: string,
  progress: Omit<SubmitProgress, "request_id">,
) => send("submit_progress", { request_id: requestId, ...progress });

function finished(
  requestId: string,
  result: Pick<RunFinished, "kind" | "run" | "submit">,
): RunFinished {
  return {
    request_id: requestId,
    finished_at: "2026-11-24T10:00:00+09:00",
    error: null,
    progress: null,
    ...result,
  };
}

const sendFinished = (
  requestId: string,
  result: Pick<RunFinished, "kind" | "run" | "submit">,
) => send("run_finished", finished(requestId, result));

afterEach(() => {
  // 画面を離れるときの stop_run と受け取りの解除が、置き換えたコマンドに届くように、先にアンマウントする。
  cleanup();
  clearMocks();
});

describe("ボタンの有効・無効", () => {
  it("待機中は、実行・テスト・提出が押せて、停止は押せない", () => {
    setup();
    expect(button("実行")).toBeEnabled();
    expect(button("テスト")).toBeEnabled();
    expect(button("提出")).toBeEnabled();
    expect(button("停止")).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "標準入力" })).toBeDisabled();
  });
});

describe("実行", () => {
  it("イベントの受け取りを始めてから run_code を呼び、戻り値より先に届いた出力も表示する", async () => {
    const { calls, draftSave } = setup({
      emitOnStart: (id) =>
        emit("run_output", {
          request_id: id,
          chunks: [{ stream: "stdout", text: "先に届いた\n" }],
        }),
    });
    await press("実行");
    const [args] = commands(calls, "run_code");
    expect(args).toEqual({
      request_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
      lesson_id: "ch04-04",
      mode: "main",
      files: FILES,
    });
    expect(output().textContent).toMatch(/── 実行 {2}\d\d:\d\d ──/);
    expect(output()).toHaveTextContent("先に届いた");
    // run_code が下書きを保存するので、待っている保存はやめる。
    expect(draftSave.cancel).toHaveBeenCalled();
    expect(draftSave.edited).not.toHaveBeenCalled();
  });

  it("実行中は、実行・テスト・提出が押せず、停止が押せる。ビルド中は入力欄が無効", async () => {
    setup();
    await press("実行");
    expect(button("実行")).toBeDisabled();
    expect(button("テスト")).toBeDisabled();
    expect(button("提出")).toBeDisabled();
    expect(button("停止")).toBeEnabled();
    expect(screen.getByRole("status")).toHaveTextContent("実行中");
    expect(screen.getByRole("textbox", { name: "標準入力" })).toBeDisabled();
  });

  it("出力を、届いた順に色を付けてコンソールに出す。ほかの要求の出力は出さない", async () => {
    const { calls } = setup();
    await press("実行");
    const id = lastRequestId(calls);
    await sendOutput(id, [
      { stream: "stderr", text: "\u001b[1m\u001b[91merror[E0382]\u001b[0m\n" },
      { stream: "stdout", text: "こんにちは\n" },
    ]);
    await sendOutput("other", [{ stream: "stdout", text: "ほかの実行\n" }]);
    expect(output()).toHaveTextContent("error[E0382]");
    expect(output()).toHaveTextContent("こんにちは");
    expect(output()).not.toHaveTextContent("ほかの実行");
    const error = screen.getByText("error[E0382]");
    expect(error).toHaveClass("pg-out-stderr", "ansi-red", "ansi-bold");
  });

  it("run_started で入力欄が有効になり、Enter で send_stdin を呼んで入力を表示し、「入力終了」で close_stdin を呼ぶ", async () => {
    const { calls } = setup();
    await press("実行");
    const id = lastRequestId(calls);
    await send("run_started", { request_id: id });
    const stdin = screen.getByRole("textbox", { name: "標準入力" });
    expect(stdin).toBeEnabled();
    fireEvent.change(stdin, { target: { value: "ferris" } });
    await act(async () => {
      fireEvent.keyDown(stdin, { key: "Enter" });
    });
    expect(commands(calls, "send_stdin")).toEqual([
      { request_id: id, text: "ferris\n" },
    ]);
    expect(screen.getByText("ferris", { exact: false })).toHaveClass(
      "pg-out-input",
    );
    await press("入力終了");
    expect(commands(calls, "close_stdin")).toEqual([{ request_id: id }]);
    expect(stdin).toBeDisabled();
  });

  it("run_finished で待機に戻り、終了コードを書く。受け取りもやめる", async () => {
    const { calls } = setup();
    await press("実行");
    const id = lastRequestId(calls);
    await sendFinished(id, {
      kind: "run",
      run: { mode: "main", status: "success", exit_code: 0 },
      submit: null,
    });
    expect(output()).toHaveTextContent("終了しました（終了コード 0）");
    expect(button("実行")).toBeEnabled();
    expect(button("停止")).toBeDisabled();
    expect(screen.queryByRole("status")).toBeNull();
    // 受け取りをやめたので、同じ ID のイベントが来ても表示しない。
    await sendOutput(id, [{ stream: "stdout", text: "遅れて届いた\n" }]);
    expect(output()).not.toHaveTextContent("遅れて届いた");
  });

  it("「停止」で stop_run を呼ぶ", async () => {
    const { calls } = setup();
    await press("実行");
    const id = lastRequestId(calls);
    await press("停止");
    expect(commands(calls, "stop_run")).toEqual([{ request_id: id }]);
    await sendFinished(id, {
      kind: "run",
      run: { mode: "main", status: "stopped", exit_code: null },
      submit: null,
    });
    expect(output()).toHaveTextContent("停止しました");
  });

  it("続けて 2 回押しても、run_code は 1 回だけ呼ぶ", async () => {
    const { calls } = setup();
    await act(async () => {
      fireEvent.click(button("実行"));
      fireEvent.click(button("実行"));
    });
    expect(commands(calls, "run_code")).toHaveLength(1);
  });

  it("busy で拒否されたら、message を出して待機に戻り、保存を待つ状態に戻す", async () => {
    const { draftSave } = setup({
      reject: {
        run_code: {
          code: "busy",
          message: "ほかの実行・提出が動いています。",
          detail: null,
        },
      },
    });
    await press("実行");
    expect(output()).toHaveTextContent("ほかの実行・提出が動いています。");
    expect(button("実行")).toBeEnabled();
    expect(button("停止")).toBeDisabled();
    expect(draftSave.edited).toHaveBeenCalled();
  });

  it("send_stdin が拒否されたら、message を出す", async () => {
    const { calls } = setup({
      reject: {
        send_stdin: {
          code: "invalid_state",
          message: "プログラムは終了しています。",
          detail: null,
        },
      },
    });
    await press("実行");
    await send("run_started", { request_id: lastRequestId(calls) });
    const stdin = screen.getByRole("textbox", { name: "標準入力" });
    await act(async () => {
      fireEvent.keyDown(stdin, { key: "Enter" });
    });
    expect(output()).toHaveTextContent("プログラムは終了しています。");
  });
});

describe("テスト", () => {
  it("mode を test にして run_code を呼び、入力欄を出さない", async () => {
    const { calls } = setup();
    await press("テスト");
    expect(commands(calls, "run_code")[0]).toMatchObject({ mode: "test" });
    expect(output().textContent).toMatch(/── テスト {2}\d\d:\d\d ──/);
    expect(screen.queryByRole("textbox", { name: "標準入力" })).toBeNull();
    expect(button("停止")).toBeEnabled();
    const id = lastRequestId(calls);
    await sendOutput(id, [
      { stream: "stdout", text: "test tests::adds ... ok\n" },
    ]);
    expect(output()).toHaveTextContent("test tests::adds ... ok");
    await sendFinished(id, {
      kind: "run",
      run: { mode: "test", status: "success", exit_code: 0 },
      submit: null,
    });
    expect(
      screen.getByRole("textbox", { name: "標準入力" }),
    ).toBeInTheDocument();
  });
});

describe("提出", () => {
  it("submit_code を呼び、提出中は停止が押せない。進み具合を出し、それまでの出力を消さない", async () => {
    const { calls, states } = setup();
    await press("実行");
    const runId = lastRequestId(calls);
    await sendOutput(runId, [{ stream: "stdout", text: "実行の出力\n" }]);
    await sendFinished(runId, {
      kind: "run",
      run: { mode: "main", status: "success", exit_code: 0 },
      submit: null,
    });

    await press("提出");
    expect(commands(calls, "submit_code")).toEqual([
      { request_id: expect.any(String), lesson_id: "ch04-04", files: FILES },
    ]);
    const id = lastRequestId(calls);
    expect(button("停止")).toBeDisabled();
    expect(button("実行")).toBeDisabled();
    expect(screen.getByRole("status")).toHaveTextContent("ビルド中");

    await sendProgress(id, { phase: "build", completed: 0, total: null });
    expect(screen.getByRole("status")).toHaveTextContent("ビルド中");
    await sendProgress(id, { phase: "judge", completed: 0, total: 3 });
    await sendProgress(id, { phase: "judge", completed: 1, total: 3 });
    expect(screen.getByRole("status")).toHaveTextContent("判定中 1 / 3");
    expect(button("停止")).toBeDisabled();

    const result = finished(id, {
      kind: "submit",
      run: null,
      submit: {
        status: "wrong",
        passed: 2,
        total: 3,
        cases: [],
      },
    });
    await send("run_finished", result);
    const text = output().textContent ?? "";
    expect(text).toContain("実行の出力");
    expect(text).toMatch(/── 提出 {2}\d\d:\d\d ──\nビルドしました\n/);
    expect(text).toContain("テストを実行しました（3 / 3）");
    expect(text).toContain("3 件中 1 件のテストが失敗しました。");
    // 「ビルドしました」は 1 回だけ書く。
    expect(text.split("ビルドしました")).toHaveLength(2);
    // 判定結果は、演習画面の状態に渡す。
    expect(states.at(-1)).toEqual({ kind: "idle" });
    expect(button("提出")).toBeEnabled();
  });

  it("正解すると、演習画面の状態が解説表示になる", async () => {
    const { calls, states } = setup();
    await press("提出");
    await sendFinished(lastRequestId(calls), {
      kind: "submit",
      run: null,
      submit: { status: "passed", passed: 3, total: 3, cases: [] },
    });
    expect(states.at(-1)).toEqual({ kind: "explanation" });
    expect(output()).toHaveTextContent("3 件のテストがすべて通りました。");
    expect(button("実行")).toBeDisabled();
    expect(button("提出")).toBeDisabled();
  });

  it("提出の run_finished を、押した時刻と submit_code に渡したファイルと一緒に知らせる。実行の run_finished は知らせない", async () => {
    const { calls, submits } = setup();
    await press("実行");
    await sendFinished(lastRequestId(calls), {
      kind: "run",
      run: { mode: "main", status: "success", exit_code: 0 },
      submit: null,
    });
    expect(submits).toEqual([]);
    const before = new Date();
    await press("提出");
    const id = lastRequestId(calls);
    const submit = {
      status: "wrong" as const,
      passed: 0,
      total: 1,
      cases: [{ name: "a", status: "fail" as const, detail: "x" }],
    };
    await sendFinished(id, { kind: "submit", run: null, submit });
    expect(submits).toHaveLength(1);
    expect(submits[0].event).toEqual(
      finished(id, { kind: "submit", run: null, submit }),
    );
    expect(submits[0].files).toEqual(FILES);
    expect(submits[0].submittedAt.getTime()).toBeGreaterThanOrEqual(
      before.getTime(),
    );
  });
});

describe("画面を離れるとき", () => {
  it.each([
    ["実行", "run_code"],
    ["テスト", "run_code"],
    ["提出", "submit_code"],
  ])("%sの途中で離れると、その ID で stop_run を呼ぶ", async (name) => {
    const { calls, unmount } = setup();
    await press(name);
    const id = lastRequestId(calls);
    unmount();
    expect(commands(calls, "stop_run")).toEqual([{ request_id: id }]);
  });

  it("待機中に離れても、stop_run を呼ばない", async () => {
    const { calls, unmount } = setup();
    unmount();
    expect(commands(calls, "stop_run")).toEqual([]);
  });

  it("run_code の戻りを待つ間に離れたら、始まった後にもう一度 stop_run を呼ぶ", async () => {
    let release = () => {};
    const hold = new Promise<void>((resolve) => {
      release = resolve;
    });
    const { calls, unmount } = setup({ hold });
    await press("実行");
    const id = lastRequestId(calls);
    unmount();
    expect(commands(calls, "stop_run")).toHaveLength(1);
    await act(async () => {
      release();
    });
    expect(commands(calls, "stop_run")).toEqual([
      { request_id: id },
      { request_id: id },
    ]);
  });

  it("run_code の戻りを待つ間に離れ、run_code が拒否されたら、下書きの保存を待つ状態に戻す", async () => {
    let release = () => {};
    const hold = new Promise<void>((resolve) => {
      release = resolve;
    });
    const { draftSave, unmount } = setup({
      hold,
      reject: {
        run_code: { code: "busy", message: "動いています。", detail: null },
      },
    });
    await press("実行");
    expect(draftSave.cancel).toHaveBeenCalled();
    unmount();
    await act(async () => {
      release();
    });
    expect(draftSave.edited).toHaveBeenCalled();
  });
});

describe("外部クレートの準備", () => {
  const buttons = () =>
    ["実行", "テスト", "提出"].map((name) =>
      screen.getByRole("button", { name }),
    );

  it.each([
    ["preparing", "外部クレートを準備しています"],
    ["failed", "外部クレートの準備に失敗しました"],
  ] as const)(
    "許可クレートを使う演習は、%s のとき案内を出し、実行・テスト・提出を無効にする",
    async (crates, text) => {
      setup({ dependencies: ["rand"], crates });
      expect(await screen.findByText(new RegExp(text))).toBeInTheDocument();
      for (const button of buttons()) {
        expect(button).toBeDisabled();
      }
    },
  );

  it.each(["preparing", "failed"] as const)(
    "許可クレートを使わない演習は、%s のときも案内を出さず、ボタンを無効にしない",
    async (crates) => {
      setup({ dependencies: [], crates });
      await act(async () => {});
      expect(screen.queryByText(/外部クレート/)).toBeNull();
      for (const button of buttons()) {
        expect(button).toBeEnabled();
      }
    },
  );

  it("ready になると、案内が消えてボタンが有効になる", async () => {
    setup({ dependencies: ["rand"], crates: "preparing" });
    await screen.findByText(/外部クレートを準備しています/);
    await act(async () => {
      await emit("environment_updated", {
        cargo: {
          status: "found",
          cargo_path: "/c/cargo",
          rustc_path: "/c/rustc",
          rustc_version: "1.100.0",
          required_rustc_version: "1.100.0",
        },
        claude: { status: "found", path: "/c/claude", version: "1.0.0" },
        clang: { status: "found", path: "/usr/bin/clang", version: "21.0.0" },
        crates: { status: "ready", prepared_at: null, error: null },
      });
    });
    expect(screen.queryByText(/外部クレートを準備/)).toBeNull();
    for (const button of buttons()) {
      expect(button).toBeEnabled();
    }
  });
});
