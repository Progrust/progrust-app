// 質疑応答のチャットパネル（documents/spec/qa-chat/screen.md、
// documents/spec/claude-integration/unavailable.md、documents/spec/claude-integration/errors.md「分類」、
// design-system/progrust/components/ChatPanel/README.md）。
// useChat が返す会話と操作を受け取って表示する。動画画面と正解のモーダルの両方に置ける。
// 見出し・空の状態の文・閉じるボタンの有無は、置く側が渡す。
import {
  ArrowRight,
  CircleAlert,
  LoaderCircle,
  TriangleAlert,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { aiErrorMessage } from "../claude/aiErrorMessage";
import type { ClaudeStatus } from "../commands/environment";
import { openInstallGuide } from "../environment/installGuide";
import { Markdown } from "../markdown/Markdown";
import type { ChatExchange, UseChat } from "./useChat";

// 入力欄の最大の文字数。
export const CHAT_INPUT_MAX_LENGTH = 2000;

export type ChatPanelProps = {
  chat: UseChat;
  // 見出し（例: 「質問する」）。
  title: string;
  // 入力欄の案内（プレースホルダーと、読み上げ用の名前）。
  placeholder: string;
  // 会話が空のときの案内。無ければ出さない。
  empty?: { title: string; text: string };
  // `claude` の検出の状態。読み込むまでは null（見つかっているものとして扱う）。
  claude: ClaudeStatus | null;
  // 閉じるボタンを押したとき。渡さなければ、閉じるボタンを出さない。
  onClose?: () => void;
  closeLabel?: string;
};

export function ChatPanel({
  chat,
  title,
  placeholder,
  empty,
  claude,
  onClose,
  closeLabel = "質問パネルを閉じる",
}: ChatPanelProps) {
  const { exchanges, answering, send, stop, retry } = chat;
  const unavailable = claude !== null && claude.status === "not_found";
  const [draft, setDraft] = useState("");
  const logRef = useRef<HTMLDivElement>(null);

  // 新しい断片が届いたら、会話の末尾が見えるようにする。
  useEffect(() => {
    const log = logRef.current;
    if (log !== null) {
      log.scrollTop = log.scrollHeight;
    }
  }, [exchanges]);

  const submit = () => {
    if (answering || unavailable || draft.trim() === "") {
      return;
    }
    if (send(draft)) {
      setDraft("");
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== "Enter" || !event.metaKey) {
      return;
    }
    // 日本語の変換中の Enter は、変換の確定であり、送信ではない。
    if (event.nativeEvent.isComposing || event.keyCode === 229) {
      return;
    }
    event.preventDefault();
    submit();
  };

  const lastIndex = exchanges.length - 1;
  const showEmpty = exchanges.length === 0;

  return (
    <aside className="pg-chat" aria-label={title}>
      <div className="pg-chat-head">
        <h2>{title}</h2>
        {onClose === undefined ? null : (
          <button
            type="button"
            className="pg-icon-btn pg-icon-btn--md"
            aria-label={closeLabel}
            onClick={onClose}
          >
            <X
              className="pg-icon"
              aria-hidden="true"
              size={15}
              strokeWidth={2.4}
            />
          </button>
        )}
      </div>
      {showEmpty && unavailable ? (
        <div className="pg-chat-empty">
          <b>質問は使えません</b>
          <p>Claude Code を導入すると、質問できます。</p>
        </div>
      ) : showEmpty && empty !== undefined ? (
        <div className="pg-chat-empty">
          <b>{empty.title}</b>
          <p>{empty.text}</p>
        </div>
      ) : (
        <div className="pg-chat-log" ref={logRef}>
          {exchanges.map((exchange, index) => (
            <Exchange
              key={index}
              exchange={exchange}
              canRetry={index === lastIndex && !answering}
              onRetry={retry}
            />
          ))}
        </div>
      )}
      {unavailable ? (
        <div className="pg-chat-unavailable" role="note">
          <TriangleAlert
            className="pg-icon"
            aria-hidden="true"
            size={16}
            strokeWidth={2.2}
          />
          <div>
            Claude Code が見つかりません。
            <br />
            <a
              href="#"
              onClick={(event) => {
                event.preventDefault();
                openClaudeGuide();
              }}
            >
              導入手順を見る
            </a>
          </div>
        </div>
      ) : answering ? (
        <div className="pg-composer">
          <span className="pg-composer-note">
            回答中は次の質問を送れません。
          </span>
          <button
            type="button"
            className="pg-btn pg-btn--outline"
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
        </div>
      ) : (
        <form
          className="pg-composer"
          onSubmit={(event) => {
            event.preventDefault();
            submit();
          }}
        >
          <div className="pg-inbox">
            <textarea
              className="pg-textarea"
              rows={2}
              maxLength={CHAT_INPUT_MAX_LENGTH}
              placeholder={placeholder}
              aria-label={placeholder}
              value={draft}
              onChange={(event) =>
                setDraft(event.target.value.slice(0, CHAT_INPUT_MAX_LENGTH))
              }
              onKeyDown={onKeyDown}
            />
            <div className="pg-inbox-foot">
              <span className="pg-inbox-hint">
                ⌘ Enter で送信・Enter で改行
              </span>
              <button
                type="submit"
                className="pg-icon-btn pg-icon-btn--ink pg-icon-btn--sm"
                aria-label="送信"
                disabled={draft.trim() === ""}
              >
                <ArrowRight
                  className="pg-icon"
                  aria-hidden="true"
                  size={16}
                  strokeWidth={2.2}
                />
              </button>
            </div>
          </div>
        </form>
      )}
    </aside>
  );
}

function Exchange({
  exchange,
  canRetry,
  onRetry,
}: {
  exchange: ChatExchange;
  canRetry: boolean;
  onRetry: () => boolean;
}) {
  return (
    <>
      <div className="pg-bubble pg-bubble--me">{exchange.question}</div>
      <Answer exchange={exchange} canRetry={canRetry} onRetry={onRetry} />
    </>
  );
}

function Answer({
  exchange,
  canRetry,
  onRetry,
}: {
  exchange: ChatExchange;
  canRetry: boolean;
  onRetry: () => boolean;
}) {
  if (exchange.status === "error" && exchange.error !== null) {
    const { message, detail } =
      exchange.error.kind === "ai"
        ? aiErrorMessage(exchange.error.error)
        : { message: exchange.error.message, detail: null };
    return (
      <div className="pg-bubble-error" role="alert">
        <div className="pg-bubble-error-title">
          <CircleAlert
            className="pg-icon"
            aria-hidden="true"
            size={14}
            strokeWidth={2.4}
          />
          {message}
        </div>
        {detail === null ? null : (
          <details>
            <summary>詳細</summary>
            <pre>{detail}</pre>
          </details>
        )}
        {canRetry ? (
          <div>
            <button
              type="button"
              className="pg-btn pg-btn--sm pg-btn--outline"
              onClick={() => onRetry()}
            >
              再試行
            </button>
          </div>
        ) : null}
      </div>
    );
  }
  const answering = exchange.status === "answering";
  if (answering && exchange.answer === "") {
    return (
      <div className="pg-bubble pg-bubble--ai pg-bubble--wait">
        <Waiting />
      </div>
    );
  }
  return (
    <div className="pg-bubble pg-bubble--ai">
      {exchange.answer === "" ? null : (
        <Markdown source={exchange.answer} highlight />
      )}
      {answering ? (
        <div className="pg-bubble--wait">
          <Waiting />
        </div>
      ) : null}
      {exchange.status === "stopped" ? (
        <p className="pg-stopped">（停止しました）</p>
      ) : null}
    </div>
  );
}

function Waiting() {
  return (
    <>
      <LoaderCircle
        className="pg-icon pg-spin"
        aria-hidden="true"
        size={14}
        strokeWidth={2.4}
      />
      回答を作成しています
    </>
  );
}

// 導入手順を既定ブラウザで開く。開けなかったときは、画面には出さず、記録だけする。
function openClaudeGuide() {
  openInstallGuide("claude").catch((error: unknown) => {
    console.error("Claude Code の導入手順を開けなかった", error);
  });
}
