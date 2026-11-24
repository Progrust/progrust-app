// 「初期コードに戻す」の確認のダイアログ（documents/spec/lesson-flow/drafts.md、
// design-system/progrust/components/Dialog）。Esc と幕のクリックで閉じ、開いたら「キャンセル」にフォーカスを移す。
import { useEffect, useRef } from "react";
import type { KeyboardEvent, MouseEvent } from "react";

export function ResetDraftDialog({
  busy,
  onCancel,
  onConfirm,
}: {
  // 戻している途中か。途中は、閉じることも、もう一度押すこともできない。
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    cancelRef.current?.focus();
  }, []);

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape" && !busy) {
      event.stopPropagation();
      onCancel();
    }
  };
  const handleScrimClick = (event: MouseEvent) => {
    if (event.target === event.currentTarget && !busy) {
      onCancel();
    }
  };

  return (
    <div
      className="pg-scrim exercise-overlay"
      onClick={handleScrimClick}
      onKeyDown={handleKeyDown}
    >
      <div
        className="pg-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="reset-draft-title"
        aria-describedby="reset-draft-desc"
      >
        <h2 id="reset-draft-title">初期コードに戻しますか？</h2>
        <p id="reset-draft-desc">
          すべてのファイルを初期コードに戻します。いまのコードは消え、元に戻せません。
        </p>
        <div className="pg-dialog-actions">
          <button
            type="button"
            className="pg-btn"
            ref={cancelRef}
            disabled={busy}
            onClick={onCancel}
          >
            キャンセル
          </button>
          <button
            type="button"
            className="pg-btn pg-btn--danger"
            disabled={busy}
            onClick={onConfirm}
          >
            初期コードに戻す
          </button>
        </div>
      </div>
    </div>
  );
}
