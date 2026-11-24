// 正解のモーダルを閉じた後に、演習画面の下に出す帯（documents/spec/screens/exercise-screen.md「正解したとき」、
// design-system/progrust/components/SolvedBar、design-system/ui-design/Exercise.dc.html）。
// 「解説を開く」でモーダルを開き直し、「レッスン一覧へ」「次のレッスンへ」で画面を移る。
import { Check } from "lucide-react";

export type SolvedBarProps = {
  // 「次のレッスンへ」に添える、次のレッスンの番号と名前。次のレッスンが無いときは null で、ボタンを出さない。
  nextLabel: string | null;
  // 画面を移っている途中か。途中は、移るボタンを押せない。
  leaving: boolean;
  onOpen: () => void;
  onLessons: () => void;
  onNext: () => void;
};

export function SolvedBar({
  nextLabel,
  leaving,
  onOpen,
  onLessons,
  onNext,
}: SolvedBarProps) {
  return (
    <div className="exercise-solved">
      <div className="pg-solved" role="status">
        <span className="pg-solved-mark">
          <Check
            className="pg-icon"
            aria-hidden="true"
            size={16}
            strokeWidth={3}
          />
        </span>
        <span className="pg-solved-text">正解しました</span>
        <span className="pg-solved-sub">
          エディタは読み取り専用になりました。
        </span>
        <div className="pg-solved-actions">
          <button
            type="button"
            className="pg-btn pg-btn--outline"
            onClick={onOpen}
          >
            解説を開く
          </button>
          <button
            type="button"
            className="pg-btn pg-btn--outline"
            disabled={leaving}
            onClick={onLessons}
          >
            レッスン一覧へ
          </button>
          {nextLabel !== null && (
            <button
              type="button"
              className="pg-btn pg-btn--accent"
              disabled={leaving}
              onClick={onNext}
            >
              次のレッスンへ
              <span className="pg-btn-sub">{nextLabel}</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
