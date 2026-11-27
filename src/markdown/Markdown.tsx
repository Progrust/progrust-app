// 教材の Markdown（問題文など）の表示（ADR 0228）。
// react-markdown で React の要素にする。生の HTML は表示せずに捨てる（skipHtml）。
// 本文の見た目はデザインシステムの ProblemPanel（.pg-prose）と CodeBlock（.pg-code）に従う。
import type { ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";

import { tokenizeRust } from "../video/highlight";

// リンクは、文字だけを表示する。押すと、アプリの画面（WebView）がリンク先に移ってしまうため。
const link: Components["a"] = ({ children }) => <>{children}</>;

// `rust` のコードブロックを、字句ごとに色分けする（ADR 0188）。インラインのコードと、ほかの言語は、そのまま出す。
// 閉じていないコードブロックも、届いている分だけを色分けする。
const highlightedCode: Components["code"] = ({ className, children }) => {
  const isRust =
    className !== undefined && /(^|\s)language-rust(\s|$)/.test(className);
  if (!isRust || typeof children !== "string") {
    return <code className={className}>{children}</code>;
  }
  const parts: ReactNode[] = tokenizeRust(children).map((token, index) =>
    token.cls === null ? (
      token.text
    ) : (
      <span key={index} className={token.cls}>
        {token.text}
      </span>
    ),
  );
  return <code className={className}>{parts}</code>;
};

function components(onBg: boolean, highlight: boolean): Components {
  return {
    // コードブロックは、デザインシステムの CodeBlock の面にする（地の面の上では pg-code--on-bg）。
    pre: ({ children }) => (
      <pre className={onBg ? "pg-code pg-code--on-bg" : "pg-code"}>
        {children}
      </pre>
    ),
    a: link,
    ...(highlight ? { code: highlightedCode } : {}),
  };
}

const PLAIN = components(false, false);
const PLAIN_ON_BG = components(true, false);
const HIGHLIGHT = components(false, true);
const HIGHLIGHT_ON_BG = components(true, true);

export function Markdown({
  source,
  onBg = false,
  highlight = false,
}: {
  source: string;
  // 地の面の上に置くか。
  onBg?: boolean;
  // `rust` のコードブロックを色分けするか。質疑応答の回答だけが使う。問題文・解説は色分けしない。
  highlight?: boolean;
}) {
  const map = highlight
    ? onBg
      ? HIGHLIGHT_ON_BG
      : HIGHLIGHT
    : onBg
      ? PLAIN_ON_BG
      : PLAIN;
  return (
    <div className={onBg ? "pg-prose pg-prose--on-bg" : "pg-prose"}>
      <ReactMarkdown skipHtml components={map}>
        {source}
      </ReactMarkdown>
    </div>
  );
}
