// 教材の Markdown（問題文など）の表示（ADR 0228）。
// react-markdown で React の要素にする。生の HTML は表示せずに捨てる（skipHtml）。
// 本文の見た目はデザインシステムの ProblemPanel（.pg-prose）と CodeBlock（.pg-code）に従う。
import ReactMarkdown, { type Components } from "react-markdown";

// リンクは、文字だけを表示する。押すと、アプリの画面（WebView）がリンク先に移ってしまうため。
const link: Components["a"] = ({ children }) => <>{children}</>;

const COMPONENTS: Components = {
  // コードブロックは、デザインシステムの CodeBlock の面にする。
  pre: ({ children }) => <pre className="pg-code">{children}</pre>,
  a: link,
};

// 地の面（bg）の上に置くとき（正解のモーダルの中）の見た目。ProblemPanel の pg-prose--on-bg と、
// CodeBlock の pg-code--on-bg。
const ON_BG_COMPONENTS: Components = {
  pre: ({ children }) => (
    <pre className="pg-code pg-code--on-bg">{children}</pre>
  ),
  a: link,
};

export function Markdown({
  source,
  onBg = false,
}: {
  source: string;
  // 地の面の上に置くか。
  onBg?: boolean;
}) {
  return (
    <div className={onBg ? "pg-prose pg-prose--on-bg" : "pg-prose"}>
      <ReactMarkdown skipHtml components={onBg ? ON_BG_COMPONENTS : COMPONENTS}>
        {source}
      </ReactMarkdown>
    </div>
  );
}
