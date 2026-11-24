// 演習画面。画面の配置は design-system/ui-design/Exercise.dc.html、
// 決まりは documents/spec/screens/exercise-screen.md、documents/spec/lesson-flow/flow.md、drafts.md。
// 教材と下書きを読み込み、左に問題文、右上にエディタ、右下にコンソールを置く。
// 下書きは、保存のきっかけ（draftSave.ts）で `save_draft` に保存し、「初期コードに戻す」で `reset_draft` を呼ぶ。
import { ChevronLeft, Pencil, Plus, RotateCcw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";
import { findLesson, numberChapters } from "../chapterNumbers";
import { getLesson, listChapters } from "../commands/content";
import type { Chapter, Exercise, LessonDetail } from "../commands/content";
import { openLesson, resetDraft, saveDraft } from "../commands/progress";
import type { DraftFiles } from "../commands/progress";
import { getSettings } from "../commands/settings";
import { Console, EMPTY_CONSOLE_LOG } from "../console";
import { CodeEditor } from "../editor";
import { Markdown } from "../markdown/Markdown";
import { useNavigation } from "../router";
import { useDraftSaveTrigger } from "./draftSave";
import { exerciseLinks } from "./exerciseLinks";
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
import { Splitter } from "./Splitter";
import "./exerciseScreen.css";

export { MAIN_FILE };

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

  const back = (event: MouseEvent) => {
    event.preventDefault();
    navigate(links.backRoute);
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
            onClick={() => navigate(reviewVideoRoute)}
          >
            動画を見直す
          </button>
        )}
      </div>
      {loaded !== null ? (
        <ExerciseWorkspace lessonId={lessonId} loaded={loaded} />
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

// 読み込んだ後の、問題文・エディタ・コンソール。
function ExerciseWorkspace({
  lessonId,
  loaded,
}: {
  lessonId: string;
  loaded: Loaded;
}) {
  // エディタ上の全ファイル。保存は、画面を離れるとき（アンマウント）にも呼ぶので、ref からも読めるようにする。
  const [files, setFiles] = useState(loaded.files);
  const filesRef = useRef(files);
  const replaceFiles = (next: DraftFiles) => {
    filesRef.current = next;
    setFiles(next);
  };

  // 下書きのコマンドを 1 つずつ送る。保存と「初期コードに戻す」の順番が入れ替わらないようにする。
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const enqueue = <T,>(task: () => Promise<T>): Promise<T> => {
    const next = queue.current.then(task);
    queue.current = next.catch(() => {});
    return next;
  };

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
  const activeReadonly = isMulti && isReadonly(activeFile, readonlyFiles);

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

  return (
    <div
      className="exercise-body"
      ref={bodyRef}
      style={{ "--exercise-side-width": `${sideWidth}px` } as object}
    >
      <div className="exercise-side">
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
          (bodyRef.current?.clientWidth ?? 0) - SPLIT_V_SIZE - EDITOR_MIN_WIDTH
        }
        direction={1}
        onChange={setSideWidth}
      />

      <div
        className="exercise-work"
        ref={workRef}
        style={{ "--exercise-console-height": `${consoleHeight}px` } as object}
      >
        <div className="pg-editor">
          <div className="pg-editor-bar">
            <div role="tablist" aria-label="ファイル" className="exercise-tabs">
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
            {isMulti && (
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
            <button
              type="button"
              className="pg-btn pg-btn--sm pg-btn--text"
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
          <div className="exercise-editor">
            <CodeEditor
              value={files[activeFile] ?? ""}
              onChange={handleChange}
              readOnly={activeReadonly}
              fontSize={loaded.fontSize}
              ariaLabel={`エディタ ${activeFile}`}
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
          <Console
            log={EMPTY_CONSOLE_LOG}
            stdin={{ enabled: false, onSend: () => {}, onClose: () => {} }}
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
  );
}
