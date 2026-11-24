// 演習画面のコンソールの部品（documents/spec/screens/exercise-screen.md、
// documents/spec/exercise-runner/run.md「対話的な標準入力」、ADR 0013、ADR 0103、ADR 0224、ADR 0226）。
// 出力（ConsoleLog）を、出力の種類と ANSI の色で色分けして表示し、下に標準入力の入力欄を置く。
// 色はデザインシステムのクラス（pg-out-*・ansi-*）で付けるので、テーマを切り替えると CSS 変数で変わる。
import {
  memo,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import type { ConsoleKind, ConsoleLog, ConsoleSegment } from "./consoleLog";
import "./console.css";

// 種類ごとの文字の色のクラス（ADR 0224）。標準出力は既定の色。
const KIND_CLASS: Record<ConsoleKind, string | null> = {
  stdout: null,
  stderr: "pg-out-stderr",
  system: "pg-out-system",
  input: "pg-out-input",
};

export const TRUNCATED_NOTICE = "古い出力を省略しました";

// segment の span のクラス。ANSI の色のクラスは、種類の色のクラスより後に定義されているので、
// 両方を付けると ANSI の色になる（ADR 0224「ANSI の色が付いた部分は、どの種類でも ANSI の色にする」）。
function segmentClassName(segment: ConsoleSegment): string | undefined {
  const classes: string[] = [];
  const kindClass = KIND_CLASS[segment.kind];
  if (kindClass !== null) classes.push(kindClass);
  if (segment.style.color !== null) classes.push(`ansi-${segment.style.color}`);
  if (segment.style.bold) classes.push("ansi-bold");
  return classes.length > 0 ? classes.join(" ") : undefined;
}

// 内容の変わらない segment は描き直さない。
const Segment = memo(function Segment({
  segment,
}: {
  segment: ConsoleSegment;
}) {
  return <span className={segmentClassName(segment)}>{segment.text}</span>;
});

// 標準入力の入力欄に渡すもの。
export type ConsoleStdin = {
  // 入力できるか。実行中（学習者のプログラムが動いている間）だけ true にする。
  // 「入力終了」の後も送らないように、呼び出し側が false にする。
  enabled: boolean;
  // Enter で、入力した 1 行に改行（\n）を付けて呼ぶ。send_stdin を呼び、コンソールに入力として足すのは呼び出し側。
  onSend: (text: string) => void;
  // 「入力終了」と Ctrl+D で呼ぶ。close_stdin を呼ぶのは呼び出し側。
  onClose: () => void;
};

export type ConsoleProps = {
  log: ConsoleLog;
  // 標準入力の入力欄。null か省略で、入力欄を表示しない（テスト実行）。
  stdin?: ConsoleStdin | null;
  // 見出しの行の右に置くもの（実行・提出・停止のボタンなど）。
  actions?: ReactNode;
};

// 一番下から、この距離（px）までにいれば、一番下にいるとみなす。
const BOTTOM_SLACK = 24;

export function Console({ log, stdin, actions }: ConsoleProps) {
  const outRef = useRef<HTMLPreElement>(null);
  // 一番下を見ているか。上に戻して読んでいる間は、出力が増えても一番下に動かさない。
  const stickRef = useRef(true);

  const handleScroll = () => {
    const out = outRef.current;
    if (out !== null) {
      stickRef.current =
        out.scrollHeight - out.scrollTop - out.clientHeight <= BOTTOM_SLACK;
    }
  };

  useLayoutEffect(() => {
    const out = outRef.current;
    if (out !== null && stickRef.current) {
      out.scrollTop = out.scrollHeight;
    }
  }, [log]);

  return (
    <section className="pg-console" aria-label="コンソール">
      <div className="pg-console-bar">
        <span className="console-title">コンソール</span>
        {actions !== undefined && (
          <div className="pg-console-actions">{actions}</div>
        )}
      </div>
      <div className="pg-console-body">
        <pre
          ref={outRef}
          className="pg-console-out"
          onScroll={handleScroll}
          data-testid="console-output"
        >
          {log.truncated && (
            <span className="pg-out-system">{`${TRUNCATED_NOTICE}\n`}</span>
          )}
          {log.segments.map((segment) => (
            <Segment key={segment.id} segment={segment} />
          ))}
        </pre>
        {stdin != null && <StdinField stdin={stdin} />}
      </div>
    </section>
  );
}

function StdinField({ stdin }: { stdin: ConsoleStdin }) {
  const { enabled, onSend, onClose } = stdin;
  const [value, setValue] = useState("");
  // 入力できなくなったら、書きかけの文字を消す（次の実行に持ち越さない）。
  const [wasEnabled, setWasEnabled] = useState(enabled);
  if (enabled !== wasEnabled) {
    setWasEnabled(enabled);
    if (!enabled) {
      setValue("");
    }
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    // 日本語の変換中のキーは扱わない。WKWebView では、変換を確定する Enter が
    // isComposing が false で、keyCode が 229 で届くことがあるので、両方を見る。
    if (event.nativeEvent.isComposing || event.keyCode === 229) {
      return;
    }
    if (!enabled) {
      return;
    }
    // Cmd+Enter・Cmd+Shift+Enter は実行・提出のショートカットなので、送らない。
    if (
      event.key === "Enter" &&
      !event.metaKey &&
      !event.ctrlKey &&
      !event.altKey
    ) {
      event.preventDefault();
      onSend(`${value}\n`);
      setValue("");
    } else if (
      event.ctrlKey &&
      !event.metaKey &&
      !event.altKey &&
      !event.shiftKey &&
      event.key.toLowerCase() === "d"
    ) {
      // macOS の入力欄の Ctrl+D（後ろの 1 文字を消す）より優先する。
      event.preventDefault();
      onClose();
    }
  };

  return (
    <label className="pg-stdin">
      <span>&gt;</span>
      <input
        aria-label="標準入力"
        value={value}
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={handleKeyDown}
        disabled={!enabled}
        placeholder={enabled ? undefined : "実行中だけ入力できます"}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
      />
      <button
        type="button"
        className="pg-btn pg-btn--sm"
        disabled={!enabled}
        title="入力終了（Ctrl D）"
        onClick={onClose}
      >
        入力終了
      </button>
    </label>
  );
}
