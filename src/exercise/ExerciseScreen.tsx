// 演習画面。画面の配置は design-system/ui-design/Exercise.dc.html、
// 決まりは documents/spec/screens/exercise-screen.md、documents/spec/lesson-flow/flow.md、drafts.md。
// 教材と下書きを読み込み、左に問題文、右上にエディタ、右下にコンソールを置く。
// 下書きは、保存のきっかけ（draftSave.ts）で `save_draft` に保存し、「初期コードに戻す」で `reset_draft` を呼ぶ。
// 実行・テスト・提出・停止と出力の表示は、コンソールの部品（ExerciseConsole.tsx）が行う。
// 提出の結果は、問題文の上の判定結果のカード（JudgeCard.tsx）に出し、正解したら正解のモーダル（PassModal.tsx）を開く。
// 実行・テスト実行・提出・停止のキーボードショートカット（shortcuts.ts）は、エディタにフォーカスがあるときは Monaco の
// コマンドで、エディタの外にあるときは window の keydown のキャプチャで受ける（ADR 0111、ADR 0225）。
// 解説表示の状態で画面を離れるときは、`end_session` で試行セッションを終える
// （documents/spec/learning-record-and-analysis/attempt-log.md「試行セッションのライフサイクル」）。
import {
  ChevronLeft,
  Lock,
  Pencil,
  Plus,
  RotateCcw,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";
import { findLesson, numberChapters } from "../chapterNumbers";
import { getLesson, listChapters } from "../commands/content";
import type { Chapter, Exercise, LessonDetail } from "../commands/content";
import {
  endSession,
  openLesson,
  resetDraft,
  saveDraft,
} from "../commands/progress";
import type { DraftFiles } from "../commands/progress";
import type { SubmitResult } from "../commands/run";
import { getSettings } from "../commands/settings";
import { CodeEditor, KeyCode, KeyMod } from "../editor";
import type { EditorCommand } from "../editor";
import { lessonRoute, nextLesson } from "../lessons/nextLesson";
import { Markdown } from "../markdown/Markdown";
import { useNavigation } from "../router";
import type { Route } from "../router";
import { useDraftSaveTrigger } from "./draftSave";
import { ExerciseConsole } from "./ExerciseConsole";
import { exerciseLinks } from "./exerciseLinks";
import type { ExerciseState } from "./exerciseState";
import { JudgeCard } from "./JudgeCard";
import { nextJudgeCard } from "./judgeResult";
import type { JudgeCardState, SubmitFinished } from "./judgeResult";
import { PassedCodeModal } from "./PassedCodeModal";
import { PassModal } from "./PassModal";
import { SolvedBar } from "./SolvedBar";
import {
  MAIN_FILE,
  addFile,
  canDelete,
  canRename,
  deleteFile,
  isReadonly,
  renameFile,
  tabOrder,
  validatePath,
} from "./fileOps";
import { ResetDraftDialog } from "./ResetDraftDialog";
import {
  SHORTCUT_ACTIONS,
  SHORTCUT_LABELS,
  isComposingKey,
  shortcutForKey,
} from "./shortcuts";
import type { ShortcutAction } from "./shortcuts";
import { Splitter } from "./Splitter";
import "./exerciseScreen.css";

export { MAIN_FILE };

// ショートカットのキー（Monaco の KeyMod と KeyCode の組み合わせ）。mac では CtrlCmd が Cmd になる。
// 判定（shortcuts.ts の shortcutForKey）と同じキーにする。
const SHORTCUT_KEYBINDINGS: { [action in ShortcutAction]: number } = {
  run: KeyMod.CtrlCmd | KeyCode.Enter,
  test: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.KeyT,
  submit: KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.Enter,
  stop: KeyMod.CtrlCmd | KeyCode.Period,
};

// 設定を読み込めなかったときの文字サイズ（settings.json の editor_font_size の既定値）。
const DEFAULT_FONT_SIZE = 14;

// 境界の初期の位置と、動かせる範囲（px）。初期の位置は画面デザインと同じ。
const SIDE_WIDTH = { initial: 380, min: 260 };
const CONSOLE_HEIGHT = { initial: 232, min: 120 };
// 境界を動かしても残す、エディタの幅と高さ（px）。exerciseScreen.css の max-width・max-height と同じ値。
const EDITOR_MIN_WIDTH = 360;
const EDITOR_MIN_HEIGHT = 160;
// 境界の取っ手の幅（.pg-split-v）と高さ（.pg-split-h）。
const SPLIT_V_SIZE = 14;
const SPLIT_H_SIZE = 12;

type Loaded = {
  chapters: Chapter[] | null;
  detail: LessonDetail;
  exercise: Exercise;
  // 開いたときのエディタの全ファイル。下書きがあれば下書き、無ければ初期コード。
  files: DraftFiles;
  fontSize: number;
  // 開いたときに、レッスンが完了済みだったか。完了済みなら「前回の正解コードを見る」を出す。
  completed: boolean;
};

type State =
  | { state: "loading" }
  | { state: "failed" }
  | { state: "ready"; loaded: Loaded };

// 教材・下書き・章の並び・設定を読み込む。`open_lesson` は、画面を開いたときに 1 回だけ呼ぶ
// （「もう一度読み込む」では、呼んでいない場合だけ呼ぶ）。章末復習レッスンでは、これが試行セッションの開始になり、
// 動画画面から来たときは、同じセッションの継続になる。
async function load(
  lessonId: string,
  opened: { current: ReturnType<typeof openLesson> | null },
): Promise<Loaded> {
  if (opened.current === null) {
    const promise = openLesson(lessonId);
    // 失敗したときは、次の読み込みで呼び直す。
    promise.catch(() => {
      if (opened.current === promise) {
        opened.current = null;
      }
    });
    opened.current = promise;
  }
  const [detail, openedLesson, chapters, settings] = await Promise.all([
    getLesson(lessonId),
    opened.current,
    // 章の並びと設定は、読めなくても演習はできるので、読めないときは無いものとして続ける。
    listChapters().then(
      ({ chapters }) => chapters,
      () => null,
    ),
    getSettings().catch(() => null),
  ]);
  if (detail.exercise === null) {
    throw new Error(`レッスン ${lessonId} に演習が無い`);
  }
  return {
    chapters,
    detail,
    exercise: detail.exercise,
    files: openedLesson.draft?.files ?? detail.exercise.starter_files,
    fontSize: settings?.editor_font_size ?? DEFAULT_FONT_SIZE,
    completed: openedLesson.progress?.status === "completed",
  };
}

export function ExerciseScreen({ lessonId }: { lessonId: string }) {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<State>({ state: "loading" });
  const opened = useRef<ReturnType<typeof openLesson> | null>(null);

  useEffect(() => {
    let cancelled = false;
    load(lessonId, opened).then(
      (loaded) => {
        if (!cancelled) {
          setState({ state: "ready", loaded });
        }
      },
      () => {
        if (!cancelled) {
          setState({ state: "failed" });
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [lessonId, attempt]);

  const retry = useCallback(() => {
    setState({ state: "loading" });
    setAttempt((n) => n + 1);
  }, []);

  const { navigate } = useNavigation();
  const loaded = state.state === "ready" ? state.loaded : null;
  const kind = loaded?.detail.lesson.kind ?? "lesson";
  const links = exerciseLinks(kind, lessonId);
  const chapters = loaded?.chapters ?? null;
  const found = useMemo(
    () =>
      chapters === null ? null : findLesson(numberChapters(chapters), lessonId),
    [chapters, lessonId],
  );

  // 正解のモーダルと帯に出す、レッスンの番号と名前と、「次のレッスンへ」で開くレッスン。
  const lessonLabel =
    found !== null
      ? `${found.lesson.number} ${found.lesson.lesson.title ?? ""}`
      : (loaded?.detail.lesson.title ?? "");
  const next = useMemo(() => {
    if (chapters === null) {
      return null;
    }
    const lesson = nextLesson(chapters, lessonId);
    if (lesson === null) {
      return null;
    }
    const numbered = findLesson(numberChapters(chapters), lesson.id);
    return {
      route: lessonRoute(lesson.id, lesson.kind),
      label: `${numbered?.lesson.number ?? ""} ${lesson.title ?? ""}`.trim(),
    };
  }, [chapters, lessonId]);

  // 画面の中のリンク・ボタンで移るとき。解説表示の状態では、試行セッションを終えてから移る（ExerciseWorkspace が
  // 差し込む）。読み込む前と、解答中は、すぐ移る。
  const leaveRef = useRef<((route: Route) => void) | null>(null);
  const leave = (route: Route) => {
    if (leaveRef.current !== null) {
      leaveRef.current(route);
    } else {
      navigate(route);
    }
  };

  const back = (event: MouseEvent) => {
    event.preventDefault();
    leave(links.backRoute);
  };
  const reviewVideoRoute = loaded === null ? null : links.reviewVideoRoute;

  return (
    <main
      className="exercise-screen"
      data-chapter={found === null ? undefined : found.chapter.color}
    >
      <div className="pg-context exercise-context">
        <a className="pg-back" href="#" onClick={back}>
          <ChevronLeft
            className="pg-icon"
            aria-hidden="true"
            size={14}
            strokeWidth={2.4}
          />
          {links.backLabel}
        </a>
        {found !== null ? (
          <>
            <span className="pg-chip">
              <span className="pg-chip-num">{found.chapter.number}</span>
              {found.chapter.chapter.title}
            </span>
            <span className="pg-context-title">
              {found.lesson.number} {found.lesson.lesson.title}
            </span>
          </>
        ) : (
          loaded !== null && (
            <span className="pg-context-title">
              {loaded.detail.lesson.title}
            </span>
          )
        )}
        {reviewVideoRoute !== null && (
          <button
            type="button"
            className="pg-btn pg-context-end"
            onClick={() => leave(reviewVideoRoute)}
          >
            動画を見直す
          </button>
        )}
      </div>
      {loaded !== null ? (
        <ExerciseWorkspace
          lessonId={lessonId}
          loaded={loaded}
          lessonLabel={lessonLabel}
          chapter={
            found === null
              ? null
              : {
                  number: found.chapter.number,
                  title: found.chapter.chapter.title,
                }
          }
          next={next}
          leaveRef={leaveRef}
        />
      ) : state.state === "failed" ? (
        <div className="exercise-message">
          <p role="alert">演習を読み込めませんでした。</p>
          <button type="button" className="pg-btn" onClick={retry}>
            もう一度読み込む
          </button>
        </div>
      ) : null}
    </main>
  );
}

// 正解した提出。
type Passed = {
  result: SubmitResult;
  // submit_code に渡したファイル（正解のモーダルの「提出したコード」）。
  files: DraftFiles;
};

// 読み込んだ後の、問題文・エディタ・コンソールと、判定結果のカード・正解のモーダル・帯。
function ExerciseWorkspace({
  lessonId,
  loaded,
  lessonLabel,
  chapter,
  next,
  leaveRef,
}: {
  lessonId: string;
  loaded: Loaded;
  lessonLabel: string;
  chapter: { number: number; title: string } | null;
  // 「次のレッスンへ」で開く画面と、ボタンに添える番号と名前。無いときは null で、ボタンを出さない。
  next: { route: Route; label: string } | null;
  // 画面の中のリンク・ボタン（左上のリンク、「動画を見直す」）で移るときの関数を、ここに差し込む。
  leaveRef: { current: ((route: Route) => void) | null };
}) {
  const { navigate } = useNavigation();
  // エディタ上の全ファイル。保存は、画面を離れるとき（アンマウント）にも呼ぶので、ref からも読めるようにする。
  const [files, setFiles] = useState(loaded.files);
  const filesRef = useRef(files);
  const replaceFiles = (next: DraftFiles) => {
    filesRef.current = next;
    setFiles(next);
  };

  // 下書きのコマンドを 1 つずつ送る。保存と「初期コードに戻す」の順番が入れ替わらないようにする。
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const enqueue = useCallback(<T,>(task: () => Promise<T>): Promise<T> => {
    const next = queue.current.then(task);
    queue.current = next.catch(() => {});
    return next;
  }, []);

  const scheduler = useDraftSaveTrigger(() => {
    const snapshot = filesRef.current;
    enqueue(() => saveDraft(lessonId, snapshot)).catch(() => {
      // 保存に失敗しても、編集は続けられる。次のきっかけで、また保存する。
    });
  });

  const isMulti = loaded.detail.lesson.exercise?.layout === "multi";
  const readonlyFiles = loaded.detail.lesson.exercise?.readonly_files ?? [];
  // 開いているファイル。消えたファイル（初期コードに戻したときなど）を指していたら、src/main.rs に戻す。
  const [selected, setSelected] = useState(MAIN_FILE);
  const activeFile = selected in files ? selected : MAIN_FILE;
  // 解説表示（正解の後）か。エディタを読み取り専用にし、ファイルの操作と「初期コードに戻す」を出さない・押せなくする
  // （documents/spec/lesson-flow/flow.md）。
  const [explaining, setExplaining] = useState(false);
  // 画面を離れるとき（アンマウント）にも読めるように、ref にも持つ。
  const explainingRef = useRef(false);
  const handleStateChange = useCallback((state: ExerciseState) => {
    explainingRef.current = state.kind === "explanation";
    setExplaining(explainingRef.current);
  }, []);
  const activeReadonly =
    explaining || (isMulti && isReadonly(activeFile, readonlyFiles));

  // 試行セッションを終える（1 回だけ）。下書きのコマンドの列に並べ、待っている保存より後に送る。
  const ended = useRef<Promise<unknown> | null>(null);
  const endOnce = useCallback(() => {
    if (ended.current === null) {
      scheduler.cancel();
      ended.current = enqueue(() => endSession(lessonId));
    }
    return ended.current;
  }, [enqueue, scheduler, lessonId]);

  // ヘッダーのメニューなど、画面の外から移ったとき（アンマウント）は、ここで試行セッションを終える。
  // 画面を離れるときの下書きの保存（useDraftSaveTrigger）より後に宣言して、保存より後に送る。
  useEffect(
    () => () => {
      if (explainingRef.current) {
        endOnce().catch(() => {});
      }
    },
    [endOnce],
  );

  // 解説表示の状態で、画面の中のボタン・リンクで移るときは、試行セッションを終えてから移る。同じレッスンを開き直したとき
  // （「動画を見直す」）に、終える前のセッションが続かないようにするためである。終えられなくても、移る。
  const [leaving, setLeaving] = useState(false);
  const leave = (route: Route) => {
    if (!explainingRef.current) {
      navigate(route);
      return;
    }
    setLeaving(true);
    endOnce().then(
      () => navigate(route),
      () => navigate(route),
    );
  };
  useEffect(() => {
    leaveRef.current = leave;
  });
  useEffect(
    () => () => {
      leaveRef.current = null;
    },
    [leaveRef],
  );

  const handleChange = (value: string) => {
    // エディタの内容を画面から差し替えたとき（初期コードに戻したとき、タブを切り替えたとき）は、編集として数えない。
    if (filesRef.current[activeFile] === value) {
      return;
    }
    replaceFiles({ ...filesRef.current, [activeFile]: value });
    scheduler.edited();
  };

  // ファイルの追加・名前変更の入力。add は新しいファイル、rename は開いているファイルの名前を入力する。
  const [nameEdit, setNameEdit] = useState<{
    kind: "add" | "rename";
    value: string;
    error: string | null;
  } | null>(null);
  const changeFiles = (next: DraftFiles) => {
    replaceFiles(next);
    scheduler.edited();
  };
  const submitName = () => {
    if (nameEdit === null) {
      return;
    }
    const path = nameEdit.value.trim();
    const others =
      nameEdit.kind === "rename"
        ? deleteFile(filesRef.current, activeFile)
        : filesRef.current;
    const error = validatePath(path, others);
    if (error !== null) {
      setNameEdit({ ...nameEdit, error });
      return;
    }
    if (nameEdit.kind === "add") {
      changeFiles(addFile(filesRef.current, path));
    } else if (path !== activeFile) {
      changeFiles(renameFile(filesRef.current, activeFile, path));
    }
    setSelected(path);
    setNameEdit(null);
  };
  const removeActive = () => {
    changeFiles(deleteFile(filesRef.current, activeFile));
    setSelected(MAIN_FILE);
    setNameEdit(null);
  };

  // 判定結果のカードと、正解した提出と、正解のモーダル・前回の正解コードのモーダルを開いているか。
  const [judge, setJudge] = useState<JudgeCardState | null>(null);
  const [passed, setPassed] = useState<Passed | null>(null);
  const [passOpen, setPassOpen] = useState(false);
  const [prevCodeOpen, setPrevCodeOpen] = useState(false);
  const handleSubmitFinished = (finished: SubmitFinished) => {
    setJudge((current) => nextJudgeCard(current, finished));
    const result = finished.event.submit;
    if (finished.event.error === null && result?.status === "passed") {
      // 提出中の編集で待っている保存をやめる。正解したセッションを終えると下書きを消すので、後から作り直さない。
      scheduler.cancel();
      setNameEdit(null);
      // 読み取り専用のエディタでは、提出したコードを見せる（ADR 0046）。提出中の編集は捨てる。
      replaceFiles(finished.files);
      setPassed({ result, files: finished.files });
      setPassOpen(true);
    }
  };

  const [resetState, setResetState] = useState<"closed" | "open" | "busy">(
    "closed",
  );
  const confirmReset = () => {
    setResetState("busy");
    // 待っている保存が、戻した後に古いコードを書かないようにする。
    scheduler.cancel();
    enqueue(() => resetDraft(lessonId)).then(
      (draft) => {
        replaceFiles(draft.files);
        setResetState("closed");
      },
      () => {
        setResetState("closed");
      },
    );
  };

  // 境界の位置。
  const [sideWidth, setSideWidth] = useState(SIDE_WIDTH.initial);
  const [consoleHeight, setConsoleHeight] = useState(CONSOLE_HEIGHT.initial);
  const bodyRef = useRef<HTMLDivElement>(null);
  const workRef = useRef<HTMLDivElement>(null);

  // キーボードショートカット。押したときの処理は、コンソールの部品（ExerciseConsole）が差し込む。
  const shortcutRef = useRef<((action: ShortcutAction) => void) | null>(null);
  // モーダルを開いている間は、ショートカットを効かせない。キーはモーダルの中の操作に渡す（ADR 0231）。
  const modalOpen =
    (passed !== null && passOpen) || prevCodeOpen || resetState !== "closed";
  const modalOpenRef = useRef(modalOpen);
  useEffect(() => {
    modalOpenRef.current = modalOpen;
  }, [modalOpen]);
  // エディタの領域。ここにフォーカスがあるときのキーは、Monaco のコマンドが受ける。
  const editorAreaRef = useRef<HTMLDivElement>(null);
  // いま押されているキーが、日本語の変換中のものか。Monaco のコマンドからも見る。
  const composingRef = useRef(false);
  useEffect(() => {
    // キャプチャで受けて、エディタの外のどこにフォーカスがあっても、ほかの処理より先に取る。
    const handleKeyDown = (event: KeyboardEvent) => {
      composingRef.current = isComposingKey(event);
      if (modalOpenRef.current) {
        return;
      }
      if (
        event.target instanceof Node &&
        editorAreaRef.current?.contains(event.target) === true
      ) {
        return;
      }
      const action = shortcutForKey(event);
      if (action === null) {
        return;
      }
      // 無効なときも、ボタンの押下やフォームの送信など、キーの既定の動作をさせない。
      event.preventDefault();
      event.stopPropagation();
      shortcutRef.current?.(action);
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, []);
  // エディタの中のキーは、Monaco のコマンドとして登録し、Monaco の既定の割り当て（行の挿入など）より優先する。
  const editorCommands = useMemo(
    (): EditorCommand[] =>
      SHORTCUT_ACTIONS.map((action) => ({
        id: `progrust.exercise.${action}`,
        label: SHORTCUT_LABELS[action],
        keybinding: SHORTCUT_KEYBINDINGS[action],
        run: () => {
          if (!composingRef.current && !modalOpenRef.current) {
            shortcutRef.current?.(action);
          }
        },
      })),
    [],
  );

  const goLessons = () => leave({ name: "lessons" });
  const goNext = () => {
    if (next !== null) {
      leave(next.route);
    }
  };

  return (
    <>
      <div
        className={
          explaining ? "exercise-body exercise-body-solved" : "exercise-body"
        }
        ref={bodyRef}
        style={{ "--exercise-side-width": `${sideWidth}px` } as object}
      >
        <div className="exercise-side">
          {judge !== null && <JudgeCard key={judge.seq} card={judge} />}
          <section className="pg-side" aria-label="問題文">
            <div className="pg-side-head">問題文</div>
            <div className="pg-side-scroll">
              <Markdown source={loaded.exercise.problem_md} />
            </div>
          </section>
        </div>

        <Splitter
          orientation="vertical"
          label="問題文とエディタの境界"
          value={sideWidth}
          min={SIDE_WIDTH.min}
          max={() =>
            (bodyRef.current?.clientWidth ?? 0) -
            SPLIT_V_SIZE -
            EDITOR_MIN_WIDTH
          }
          direction={1}
          onChange={setSideWidth}
        />

        <div
          className="exercise-work"
          ref={workRef}
          style={
            { "--exercise-console-height": `${consoleHeight}px` } as object
          }
        >
          <div className="pg-editor">
            <div className="pg-editor-bar">
              <div
                role="tablist"
                aria-label="ファイル"
                className="exercise-tabs"
              >
                {(isMulti ? tabOrder(files) : [MAIN_FILE]).map((path) => (
                  <button
                    key={path}
                    type="button"
                    className="pg-filetab"
                    role="tab"
                    aria-selected={path === activeFile}
                    onClick={() => {
                      setSelected(path);
                      setNameEdit(null);
                    }}
                  >
                    {path}
                  </button>
                ))}
              </div>
              {isMulti && !explaining && (
                <>
                  <button
                    type="button"
                    className="pg-icon-btn pg-icon-btn--sm"
                    aria-label="ファイルを追加"
                    onClick={() =>
                      setNameEdit({ kind: "add", value: "src/", error: null })
                    }
                  >
                    <Plus
                      className="pg-icon"
                      aria-hidden="true"
                      size={14}
                      strokeWidth={2.4}
                    />
                  </button>
                  {canRename(activeFile, readonlyFiles) && (
                    <button
                      type="button"
                      className="pg-icon-btn pg-icon-btn--sm"
                      aria-label="ファイルの名前を変える"
                      onClick={() =>
                        setNameEdit({
                          kind: "rename",
                          value: activeFile,
                          error: null,
                        })
                      }
                    >
                      <Pencil
                        className="pg-icon"
                        aria-hidden="true"
                        size={14}
                        strokeWidth={2.4}
                      />
                    </button>
                  )}
                  {canDelete(activeFile, readonlyFiles) && (
                    <button
                      type="button"
                      className="pg-icon-btn pg-icon-btn--sm"
                      aria-label="ファイルを削除"
                      onClick={removeActive}
                    >
                      <Trash2
                        className="pg-icon"
                        aria-hidden="true"
                        size={14}
                        strokeWidth={2.4}
                      />
                    </button>
                  )}
                </>
              )}
              <div className="exercise-editor-end">
                {explaining && (
                  <span className="pg-tag exercise-readonly-tag">
                    <Lock
                      className="pg-icon"
                      aria-hidden="true"
                      size={11}
                      strokeWidth={2.6}
                    />
                    読み取り専用
                  </span>
                )}
                {loaded.completed && (
                  <button
                    type="button"
                    className="pg-btn pg-btn--sm pg-btn--text"
                    onClick={() => setPrevCodeOpen(true)}
                  >
                    前回の正解コードを見る
                  </button>
                )}
                <button
                  type="button"
                  className="pg-btn pg-btn--sm pg-btn--text"
                  disabled={explaining}
                  onClick={() => setResetState("open")}
                >
                  <RotateCcw
                    className="pg-icon"
                    aria-hidden="true"
                    size={13}
                    strokeWidth={2.4}
                  />
                  初期コードに戻す
                </button>
              </div>
            </div>
            {nameEdit !== null && (
              <form
                className="exercise-nameform"
                onSubmit={(event) => {
                  event.preventDefault();
                  submitName();
                }}
              >
                <input
                  className="pg-input pg-input--mono"
                  aria-label={
                    nameEdit.kind === "add"
                      ? "新しいファイルのパス"
                      : "新しいパス"
                  }
                  aria-invalid={nameEdit.error !== null}
                  value={nameEdit.value}
                  autoFocus
                  onChange={(event) =>
                    setNameEdit({
                      ...nameEdit,
                      value: event.target.value,
                      error: null,
                    })
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Escape") {
                      setNameEdit(null);
                    }
                  }}
                />
                <button type="submit" className="pg-btn pg-btn--sm">
                  {nameEdit.kind === "add" ? "追加" : "変更"}
                </button>
                <button
                  type="button"
                  className="pg-btn pg-btn--sm pg-btn--text"
                  onClick={() => setNameEdit(null)}
                >
                  やめる
                </button>
                {nameEdit.error !== null && (
                  <p role="alert" className="exercise-nameform-error">
                    {nameEdit.error}
                  </p>
                )}
              </form>
            )}
            <div className="exercise-editor" ref={editorAreaRef}>
              <CodeEditor
                value={files[activeFile] ?? ""}
                onChange={handleChange}
                readOnly={activeReadonly}
                fontSize={loaded.fontSize}
                ariaLabel={`エディタ ${activeFile}`}
                commands={editorCommands}
              />
            </div>
          </div>

          <Splitter
            orientation="horizontal"
            label="エディタとコンソールの境界"
            value={consoleHeight}
            min={CONSOLE_HEIGHT.min}
            max={() =>
              (workRef.current?.clientHeight ?? 0) -
              SPLIT_H_SIZE -
              EDITOR_MIN_HEIGHT
            }
            direction={-1}
            onChange={setConsoleHeight}
          />

          <div className="exercise-bottom">
            <ExerciseConsole
              lessonId={lessonId}
              dependencies={loaded.detail.lesson.exercise?.dependencies ?? []}
              getFiles={() => filesRef.current}
              enqueue={enqueue}
              draftSave={scheduler}
              onStateChange={handleStateChange}
              onSubmitFinished={handleSubmitFinished}
              shortcutRef={shortcutRef}
            />
          </div>
        </div>

        {resetState !== "closed" && (
          <ResetDraftDialog
            busy={resetState === "busy"}
            onCancel={() => setResetState("closed")}
            onConfirm={confirmReset}
          />
        )}
      </div>

      {passed !== null && (
        <SolvedBar
          nextLabel={next?.label ?? null}
          leaving={leaving}
          onOpen={() => setPassOpen(true)}
          onLessons={goLessons}
          onNext={goNext}
        />
      )}
      {passed !== null && passOpen && (
        <PassModal
          total={passed.result.total}
          lessonLabel={lessonLabel}
          chapter={chapter}
          explanationMd={loaded.exercise.explanation_md}
          problemMd={loaded.exercise.problem_md}
          files={passed.files}
          nextLabel={next?.label ?? null}
          leaving={leaving}
          onClose={() => setPassOpen(false)}
          onLessons={goLessons}
          onNext={goNext}
        />
      )}
      {prevCodeOpen && (
        <PassedCodeModal
          lessonId={lessonId}
          fontSize={loaded.fontSize}
          onClose={() => setPrevCodeOpen(false)}
        />
      )}
    </>
  );
}
