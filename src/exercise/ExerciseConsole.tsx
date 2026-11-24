// 演習画面のコンソールと、「▶ 実行」「テスト」「提出」「■ 停止」のボタン
// （documents/spec/screens/exercise-screen.md、documents/spec/architecture/commands/run.md、
// documents/spec/exercise-runner/run.md「対話的な標準入力」「停止」、documents/spec/exercise-runner/submit.md、
// ADR 0103、ADR 0112、ADR 0201、ADR 0202、ADR 0227）。
// ボタンを押すと、request_id を作り、イベントの受け取りを始めてから、run_code・submit_code を呼ぶ。
// 出力は、出力が届くたびに描き直すので、問題文とエディタを描き直さないように、この部品の中に持つ。
import { LoaderCircle } from "lucide-react";
import { useEffect, useReducer, useRef, useState } from "react";
import {
  closeStdin,
  commandErrorMessage,
  listenRunEvents,
  runCode,
  sendStdin,
  stopRun,
  submitCode,
} from "../commands/run";
import type { RunEventHandlers, SubmitProgress } from "../commands/run";
import type { DraftFiles } from "../commands/progress";
import {
  Console,
  EMPTY_CONSOLE_LOG,
  appendInput,
  appendLine,
  appendRunOutput,
} from "../console";
import type { ConsoleLog } from "../console";
import {
  BUILT_LINE,
  busyText,
  finishedLines,
  separatorLine,
} from "./consoleMessages";
import type { ConsoleLine, StartKind } from "./consoleMessages";
import {
  INITIAL_EXERCISE_STATE,
  activeRequestId,
  exerciseControls,
  exerciseReducer,
} from "./exerciseState";
import type { ExerciseState } from "./exerciseState";
import type { SubmitFinished } from "./judgeResult";
import { shortcutEnabled } from "./shortcuts";
import type { ShortcutAction } from "./shortcuts";
import { useStopOnLeave } from "./useStopOnLeave";

export type ExerciseConsoleProps = {
  lessonId: string;
  // エディタ上の全ファイル。コマンドを送る直前に読む。
  getFiles: () => DraftFiles;
  // 下書きのコマンドの列に、run_code・submit_code を並べる。前の保存より後に送るためである。
  enqueue: <T>(task: () => Promise<T>) => Promise<T>;
  // 下書きを保存するきっかけ。run_code・submit_code が下書きを保存するので、押したときに待っている保存をやめ、
  // 始められなかったときは、保存を待つ状態に戻す。
  draftSave: { cancel: () => void; edited: () => void };
  // 演習画面の状態が変わったとき（エディタを読み取り専用にするかを決めるため）。
  onStateChange: (state: ExerciseState) => void;
  // 提出の run_finished が届いたとき（判定結果のカードと正解のモーダルのため）。押した時刻と、送ったファイルを添える。
  onSubmitFinished: (finished: SubmitFinished) => void;
  // キーボードショートカットを押したときの関数を、ここに差し込む。ボタンが有効なときだけ、ボタンと同じことをする。
  shortcutRef: { current: ((action: ShortcutAction) => void) | null };
};

// cargo が見つかっているか。環境の状態はまだ読まないので、見つかっているものとする。
const CARGO_FOUND = true;

function appendLines(log: ConsoleLog, lines: ConsoleLine[]): ConsoleLog {
  return lines.reduce(
    (current, line) =>
      appendLine(current, line.kind, line.text, { style: line.style }),
    log,
  );
}

export function ExerciseConsole({
  lessonId,
  getFiles,
  enqueue,
  draftSave,
  onStateChange,
  onSubmitFinished,
  shortcutRef,
}: ExerciseConsoleProps) {
  const [state, dispatch] = useReducer(exerciseReducer, INITIAL_EXERCISE_STATE);
  const [log, setLog] = useState(EMPTY_CONSOLE_LOG);
  // 提出で最後に受け取った submit_progress。
  const [progress, setProgress] = useState<SubmitProgress | null>(null);
  // 押してから、結果が届くか始められなかったと分かるまで true。続けて押しても 2 つ目を始めない。
  const busyRef = useRef(false);
  // 動いている要求のイベントの受け取りをやめる関数。
  const unlistenRef = useRef<(() => void) | null>(null);
  // 画面を離れた（アンマウントした）か。
  const leftRef = useRef(false);
  // 動いている提出の、押した時刻と、submit_code に渡したファイル。
  const submitRef = useRef<{ submittedAt: Date; files: DraftFiles } | null>(
    null,
  );

  const controls = exerciseControls(state, CARGO_FOUND);
  const requestId = activeRequestId(state);

  useEffect(() => {
    onStateChange(state);
  }, [state, onStateChange]);

  useEffect(() => {
    leftRef.current = false;
    return () => {
      leftRef.current = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, []);

  // 画面を離れるとき、動いている実行・テスト実行・提出を止める。確認は出さない。
  useStopOnLeave(requestId, (id) => {
    stopRun(id).catch(() => {});
  });

  const showError = (error: unknown) => {
    setLog((current) =>
      appendLine(current, "system", commandErrorMessage(error)),
    );
  };

  const handlersFor = (id: string): RunEventHandlers => {
    // 提出で「ビルドしました」を書いたか。
    let built = false;
    return {
      started: () => dispatch({ type: "program_started", requestId: id }),
      output: (event) => setLog((current) => appendRunOutput(current, event)),
      progress: (event) => {
        setProgress(event);
        if (event.phase === "judge" && !built) {
          built = true;
          setLog((current) => appendLine(current, "progress", BUILT_LINE));
        }
      },
      finished: (event) => {
        unlistenRef.current?.();
        unlistenRef.current = null;
        busyRef.current = false;
        setProgress(null);
        setLog((current) => appendLines(current, finishedLines(event)));
        dispatch({ type: "finished", event });
        const submitted = submitRef.current;
        if (event.kind === "submit" && submitted !== null) {
          submitRef.current = null;
          onSubmitFinished({ event, ...submitted });
        }
      },
    };
  };

  const start = async (startKind: StartKind) => {
    if (busyRef.current) {
      return;
    }
    busyRef.current = true;
    const id = crypto.randomUUID();
    const startedAt = new Date();
    submitRef.current = null;
    dispatch(
      startKind.kind === "submit"
        ? { type: "started", kind: "submit", requestId: id }
        : { type: "started", kind: "run", mode: startKind.mode, requestId: id },
    );
    setProgress(null);
    setLog((current) =>
      appendLine(current, "system", separatorLine(startKind, startedAt), {
        blankLineBefore: true,
      }),
    );

    const failed = (error: unknown) => {
      busyRef.current = false;
      dispatch({ type: "start_failed", requestId: id });
      showError(error);
    };

    let unlisten: () => void;
    try {
      unlisten = await listenRunEvents(id, handlersFor(id));
    } catch (error) {
      if (!leftRef.current) {
        failed(error);
      }
      return;
    }
    if (leftRef.current) {
      unlisten();
      return;
    }
    unlistenRef.current = unlisten;

    try {
      await enqueue(() => {
        // 送るファイルを読むときに、待っている保存をやめる。この後の編集は、また保存を待つ。
        draftSave.cancel();
        const files = getFiles();
        if (startKind.kind === "submit") {
          submitRef.current = { submittedAt: startedAt, files };
          return submitCode(id, lessonId, files);
        }
        return runCode(id, lessonId, startKind.mode, files);
      });
    } catch (error) {
      if (unlistenRef.current === unlisten) {
        unlisten();
        unlistenRef.current = null;
      }
      // 拒否されたときは下書きを保存していないので、画面を離れた後でも、保存を待つ状態に戻す。
      draftSave.edited();
      if (!leftRef.current) {
        failed(error);
      }
      return;
    }
    // 始まる前に画面を離れていたら、離れたときの stop_run は、まだ始まっていない ID に届いて何もしなかった。
    // 始まったいま、もう一度止める。
    if (leftRef.current) {
      stopRun(id).catch(() => {});
    }
  };

  const stop = () => {
    if (requestId !== null && controls.stopEnabled) {
      stopRun(requestId).catch(showError);
    }
  };

  // ショートカットは、ボタンと同じく、有効なときだけ動かす（無効なときは何もしない）。
  const shortcut = (action: ShortcutAction) => {
    if (!shortcutEnabled(action, controls)) {
      return;
    }
    if (action === "stop") {
      stop();
    } else if (action === "submit") {
      void start({ kind: "submit" });
    } else {
      void start({ kind: "run", mode: action === "run" ? "main" : "test" });
    }
  };
  useEffect(() => {
    shortcutRef.current = shortcut;
  });
  useEffect(
    () => () => {
      shortcutRef.current = null;
    },
    [shortcutRef],
  );

  const running = state.kind === "running" ? state : null;
  const stdin = controls.stdinVisible
    ? {
        enabled: controls.stdinEnabled,
        onSend: (text: string) => {
          if (running === null) {
            return;
          }
          setLog((current) => appendInput(current, text));
          sendStdin(running.requestId, text).catch(showError);
        },
        onClose: () => {
          if (running === null) {
            return;
          }
          dispatch({ type: "stdin_closed", requestId: running.requestId });
          closeStdin(running.requestId).catch(showError);
        },
      }
    : null;

  const busy =
    state.kind === "running" ||
    state.kind === "testing" ||
    state.kind === "submitting"
      ? busyText(state.kind, progress)
      : null;

  return (
    <Console
      log={log}
      stdin={stdin}
      actions={
        <>
          {busy !== null && (
            <span className="pg-running" role="status">
              <LoaderCircle
                className="pg-icon pg-spin"
                aria-hidden="true"
                size={14}
                strokeWidth={2.4}
              />
              {busy}
            </span>
          )}
          <button
            type="button"
            className="pg-btn pg-btn--outline"
            disabled={!controls.runEnabled}
            onClick={() => void start({ kind: "run", mode: "main" })}
          >
            <svg
              className="pg-icon"
              aria-hidden="true"
              width="10"
              height="10"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M6 4l14 8-14 8z" />
            </svg>
            実行
          </button>
          <button
            type="button"
            className="pg-btn pg-btn--outline"
            disabled={!controls.testEnabled}
            onClick={() => void start({ kind: "run", mode: "test" })}
          >
            テスト
          </button>
          <button
            type="button"
            className="pg-btn pg-btn--ink"
            disabled={!controls.submitEnabled}
            onClick={() => void start({ kind: "submit" })}
          >
            提出
          </button>
          <button
            type="button"
            className={
              controls.stopEnabled ? "pg-btn pg-btn--outline" : "pg-btn"
            }
            disabled={!controls.stopEnabled}
            onClick={stop}
          >
            <svg
              className="pg-icon"
              aria-hidden="true"
              width="10"
              height="10"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <rect x="5" y="5" width="14" height="14" rx="2" />
            </svg>
            停止
          </button>
        </>
      }
    />
  );
}
