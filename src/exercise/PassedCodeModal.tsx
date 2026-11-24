// 「前回の正解コードを見る」のモーダル（documents/spec/lesson-flow/drafts.md、
// documents/spec/architecture/commands/progress.md「get_passed_code」、
// design-system/progrust/components/CodeModal、design-system/ui-design/Exercise.dc.html）。
// 開いたら get_passed_code を呼び、正解したコードを読み取り専用のエディタで見せる。編集中のコードには影響しない。
// Esc、幕のクリック、×、「閉じる」で閉じる。
import { X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import { getPassedCode } from "../commands/progress";
import type { PassedCode } from "../commands/progress";
import { CodeEditor } from "../editor";
import { passedAtText } from "./judgeResult";
import { MAIN_FILE, tabOrder } from "./fileOps";

type State =
  | { state: "loading" }
  | { state: "failed" }
  | { state: "ready"; code: PassedCode };

export function PassedCodeModal({
  lessonId,
  fontSize,
  onClose,
}: {
  lessonId: string;
  fontSize: number;
  onClose: () => void;
}) {
  const [state, setState] = useState<State>({ state: "loading" });
  const [selected, setSelected] = useState(MAIN_FILE);
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeRef.current?.focus();
  }, []);

  useEffect(() => {
    let cancelled = false;
    getPassedCode(lessonId).then(
      (code) => {
        if (!cancelled) {
          setState({ state: "ready", code });
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
  }, [lessonId]);

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
    }
  };
  const handleScrimClick = (event: MouseEvent) => {
    if (event.target === event.currentTarget) {
      onClose();
    }
  };

  const code = state.state === "ready" ? state.code : null;
  const paths = code === null ? [] : tabOrder(code.files);
  const active =
    code !== null && selected in code.files ? selected : (paths[0] ?? null);

  return (
    <div
      className="pg-scrim exercise-overlay"
      onClick={handleScrimClick}
      onKeyDown={handleKeyDown}
    >
      <div
        className="pg-code-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="prev-title"
      >
        <div className="pg-code-modal-head">
          <div>
            <h2 id="prev-title">前回の正解コード</h2>
            <p>
              {code === null
                ? "読み取り専用"
                : `${passedAtText(code.submitted_at)} ・ 読み取り専用`}
            </p>
          </div>
          <button
            type="button"
            className="pg-icon-btn pg-icon-btn--md"
            aria-label="閉じる"
            ref={closeRef}
            onClick={onClose}
          >
            <X
              className="pg-icon"
              aria-hidden="true"
              size={16}
              strokeWidth={2.2}
            />
          </button>
        </div>
        <div className="pg-editor">
          {code !== null && active !== null ? (
            <>
              <div
                className="pg-editor-bar"
                role="tablist"
                aria-label="前回の正解コードのファイル"
              >
                {paths.map((path) => (
                  <button
                    key={path}
                    type="button"
                    className="pg-filetab"
                    role="tab"
                    aria-selected={path === active}
                    onClick={() => setSelected(path)}
                  >
                    {path}
                  </button>
                ))}
              </div>
              <div className="exercise-editor">
                <CodeEditor
                  value={code.files[active] ?? ""}
                  readOnly
                  fontSize={fontSize}
                  ariaLabel={`前回の正解コード ${active}`}
                />
              </div>
            </>
          ) : (
            <div className="exercise-message">
              {state.state === "failed" ? (
                <p role="alert">前回の正解コードを読み込めませんでした。</p>
              ) : (
                <p>読み込んでいます</p>
              )}
            </div>
          )}
        </div>
        <div className="pg-code-modal-foot">
          <button type="button" className="pg-btn" onClick={onClose}>
            閉じる
          </button>
        </div>
      </div>
    </div>
  );
}
