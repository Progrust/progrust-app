// 教材の Markdown（問題文など）の表示（ADR 0228）。
// react-markdown で React の要素にする。生の HTML は表示せずに捨てる（skipHtml）。
// 本文の見た目はデザインシステムの ProblemPanel（.pg-prose）と CodeBlock（.pg-code）に従う。
import ReactMarkdown, { type Components } from "react-markdown";

const COMPONENTS: Components = {
  // コードブロックは、デザインシステムの CodeBlock の面にする。
  pre: ({ children }) => <pre className="pg-code">{children}</pre>,
  // リンクは、文字だけを表示する。押すと、アプリの画面（WebView）がリンク先に移ってしまうため。
  a: ({ children }) => <>{children}</>,
};

export function Markdown({ source }: { source: string }) {
  return (
    <div className="pg-prose">
      <ReactMarkdown skipHtml components={COMPONENTS}>
        {source}
      </ReactMarkdown>
    </div>
  );
}
