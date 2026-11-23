// コードとコンパイラの出力の表示（documents/spec/content-format/video/code-display.md）。
// 台本の code_blocks の 1 件を、block で指して表示する。
// 出力する HTML の形は design-system/progrust-video/README.md「共通部品が出力する HTML の形」の 3・4。
import type { ReactNode } from "react";
import { useCodeBlocks, type CodeBlockData } from "./hooks";
import {
  splitLines,
  tokenizeOutput,
  tokenizeRust,
  type Token,
  type OutputToken,
} from "./highlight";

export type CodeBlockProps = {
  // 表示する code_blocks の id。
  block: string;
};

type Line = {
  text: string;
  kind: "add" | "del" | "keep";
  // 表示する行番号。番号を表示しない行は null。
  number: number | null;
};

// code を行に分ける。Diff では、各行の先頭の 1 字を取り除く。
export function parseLines(block: CodeBlockData): Line[] {
  const raw = block.code.split("\n");
  const numbered = block.language !== "text";
  let next = block.start_line ?? 1;
  return raw.map((row): Line => {
    let kind: Line["kind"] = "keep";
    let text = row;
    if (block.diff === true && row !== "") {
      const sign = row[0];
      if (sign === "+") kind = "add";
      else if (sign === "-") kind = "del";
      else if (sign !== " ") {
        throw new Error(
          `code_blocks ${block.id}: Diff の行の先頭が +・-・半角空白のどれでもない: ${row}`,
        );
      }
      text = row.slice(1);
    }
    if (!numbered || kind === "del") {
      return { text, kind, number: null };
    }
    return { text, kind, number: next++ };
  });
}

function renderTokens(tokens: readonly (Token | OutputToken)[]): ReactNode {
  return tokens.map((t, i) =>
    t.cls === null ? (
      t.text
    ) : (
      <span key={i} className={t.cls}>
        {t.text}
      </span>
    ),
  );
}

export function CodeBlock({ block }: CodeBlockProps) {
  const blocks = useCodeBlocks();
  const data = blocks.find((b) => b.id === block);
  if (data === undefined) {
    throw new Error(`CodeBlock: code_blocks に id ${block} がない`);
  }

  const lines = parseLines(data);
  const bodies: ReactNode[] =
    data.language === "rust"
      ? splitLines(tokenizeRust(lines.map((l) => l.text).join("\n"))).map(
          renderTokens,
        )
      : data.language === "text"
        ? tokenizeOutput(lines.map((l) => l.text)).map(renderTokens)
        : lines.map((l) => l.text);

  return (
    <pre data-label={data.label}>
      <code
        className={`language-${data.language}`}
        data-diff={data.diff === true ? "" : undefined}
      >
        {lines.map((line, i) => (
          <span
            key={i}
            className={
              line.kind === "add"
                ? "line is-add"
                : line.kind === "del"
                  ? "line is-del"
                  : "line"
            }
            data-ln={line.number === null ? "" : String(line.number)}
          >
            {bodies[i]}
          </span>
        ))}
      </code>
    </pre>
  );
}
