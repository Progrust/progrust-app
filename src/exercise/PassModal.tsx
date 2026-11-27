// 正解のモーダル（documents/spec/screens/exercise-screen.md「正解したとき」、ADR 0046、
// design-system/progrust/components/PassModal、design-system/ui-design/Exercise.dc.html）。
// 左に解説と「問題文」「提出したコード」のタブ、右に解説への質疑応答（チャットパネル。会話は演習画面が持つ）、
// 下に「閉じてコードを見る」「レッスン一覧へ」「次のレッスンへ」を置く。Esc と × で閉じる。
// 章カラーが効くように、演習画面（data-chapter の付いた main）の中に置く。
import { ArrowRight, Check, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import type { DraftFiles } from "../commands/progress";
import { ChatPanel } from "../chat/ChatPanel";
import type { UseChat } from "../chat/useChat";
import type { ClaudeStatus } from "../commands/environment";
import { Markdown } from "../markdown/Markdown";
import { tabOrder } from "./fileOps";

export type PassTab = "explanation" | "problem" | "code";

const TABS: { id: PassTab; label: string }[] = [
  { id: "explanation", label: "解説" },
  { id: "problem", label: "問題文" },
  { id: "code", label: "提出したコード" },
];

export type PassModalProps = {
  // 通ったテストケースの数。
  total: number;
  // 「4-4 所有権のムーブ」のようなレッスンの番号と名前。
  lessonLabel: string;
  // 章のチップ。章の並びを読めなかったときは null。
  chapter: { number: number; title: string } | null;
  explanationMd: string;
  problemMd: string;
  // 提出したコード。
  files: DraftFiles;
  // 「次のレッスンへ」に添える、次のレッスンの番号と名前。次のレッスンが無いときは null で、ボタンを出さない。
  nextLabel: string | null;
  // 解説についての質疑応答の会話。モーダルを閉じても残すため、演習画面が持つ。
  chat: UseChat;
  // `claude` の検出の状態。読み込むまでは null。
  claude: ClaudeStatus | null;
  // 画面を移っている途中か。途中は、移るボタンを押せない。
  leaving: boolean;
  onClose: () => void;
  onLessons: () => void;
  onNext: () => void;
};

export function PassModal({
  total,
  lessonLabel,
  chapter,
  explanationMd,
  problemMd,
  files,
  nextLabel,
  chat,
  claude,
  leaving,
  onClose,
  onLessons,
  onNext,
}: PassModalProps) {
  const [tab, setTab] = useState<PassTab>("explanation");
  const firstTabRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    firstTabRef.current?.focus();
  }, []);

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
    }
  };

  return (
    <div className="pg-scrim exercise-overlay" onKeyDown={handleKeyDown}>
      <div
        className="pg-pass"
        role="dialog"
        aria-modal="true"
        aria-labelledby="pass-title"
      >
        <div className="pg-pass-head">
          <span className="pg-pass-mark">
            <Check
              className="pg-icon"
              aria-hidden="true"
              size={26}
              strokeWidth={3}
            />
          </span>
          <div className="exercise-pass-head-text">
            <h2 className="pg-pass-title" id="pass-title">
              正解
            </h2>
            <p className="pg-pass-sub">
              {total} 件のテストがすべて通りました。{lessonLabel}{" "}
              を完了しました。
            </p>
          </div>
          {chapter !== null && (
            <span className="pg-chip">
              <span className="pg-chip-num">{chapter.number}</span>
              {chapter.title}
            </span>
          )}
          <button
            type="button"
            className="pg-icon-btn pg-icon-btn--md"
            aria-label="閉じてコードを見る"
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
        <div className="pg-pass-body">
          <div className="pg-pass-main">
            <div className="pg-pass-tabs">
              <div className="pg-tabs" role="tablist" aria-label="正解後の表示">
                {TABS.map((item, index) => (
                  <button
                    key={item.id}
                    type="button"
                    className="pg-tab"
                    role="tab"
                    id={`pass-tab-${item.id}`}
                    aria-selected={tab === item.id}
                    aria-controls="pass-panel"
                    ref={index === 0 ? firstTabRef : undefined}
                    onClick={() => setTab(item.id)}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
            <div
              className="pg-pass-scroll"
              role="tabpanel"
              id="pass-panel"
              aria-labelledby={`pass-tab-${tab}`}
            >
              {tab === "explanation" ? (
                <Markdown source={explanationMd} onBg />
              ) : tab === "problem" ? (
                <Markdown source={problemMd} onBg />
              ) : (
                <SubmittedCode files={files} />
              )}
            </div>
          </div>
          <ChatPanel
            chat={chat}
            title="解説について質問する"
            placeholder="解説や自分のコードについて質問"
            empty={{
              title: "解説について質問できます",
              text: "自分のコードと模範解答の違い、より Rust らしい書き方など、気になったことを聞いてみましょう。",
            }}
            claude={claude}
          />
        </div>
        <div className="pg-pass-foot">
          <button
            type="button"
            className="pg-btn pg-btn--text"
            onClick={onClose}
          >
            閉じてコードを見る
          </button>
          <button
            type="button"
            className="pg-btn pg-btn--lg"
            disabled={leaving}
            onClick={onLessons}
          >
            レッスン一覧へ
          </button>
          {nextLabel !== null && (
            <button
              type="button"
              className="pg-btn pg-btn--accent pg-btn--xl"
              disabled={leaving}
              onClick={onNext}
            >
              次のレッスンへ
              <span className="pg-btn-sub">{nextLabel}</span>
              <ArrowRight
                className="pg-icon"
                aria-hidden="true"
                size={16}
                strokeWidth={2.4}
              />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// 提出したコード。ファイルごとに、パスとコードを並べる。
function SubmittedCode({ files }: { files: DraftFiles }) {
  return (
    <div className="pg-prose pg-prose--on-bg">
      {tabOrder(files).map((path) => (
        <section key={path} aria-label={path}>
          <h2>{path}</h2>
          <pre className="pg-code pg-code--on-bg">{files[path]}</pre>
        </section>
      ))}
    </div>
  );
}
