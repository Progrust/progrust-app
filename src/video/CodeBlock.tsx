// コードとコンパイラの出力の表示（documents/spec/content-format/video/code-display.md）。
// 台本の code_blocks の 1 件を、block で指して表示する。
// 出力する HTML の形は design-system/progrust-video/README.md「共通部品が出力する HTML の形」の 3〜7。
import type { CSSProperties, ReactNode } from "react";
import { useCodeBlocks, type CodeBlockData } from "./hooks";
import {
  checkLines,
  checkScopes,
  displayNumbers,
  lineKeys,
  locate,
  rowHeights,
  scopeSpan,
  type LinesSpec,
  type MarkSpec,
  type Range,
  type ScopeSpec,
} from "./codeSpec";
import { clamp01 } from "./motion";
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
  // 変数が生きている範囲の枠。
  scopes?: readonly ScopeSpec[];
  // 字句の強調（マーカー）。
  marks?: readonly MarkSpec[];
  // エラーの箇所の波線。
  errors?: readonly MarkSpec[];
  // 行ごとの動き（差し込み・入力・淡くする）。
  lines?: LinesSpec;
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

type Piece = {
  text: string;
  cls: string | null;
  mark: number;
  error: number;
};

// 行の字句を、マーカーと波線の境目で切る。入力の途中では、入力した分までにする。
function cutPieces(
  tokens: readonly (Token | OutputToken)[],
  marks: readonly Range[],
  errors: readonly Range[],
  visible: number,
): Piece[] {
  const cover = (ranges: readonly Range[], at: number) =>
    ranges.findIndex((r) => r.start <= at && at < r.end);
  const cuts = new Set<number>();
  for (const r of [...marks, ...errors]) {
    cuts.add(r.start);
    cuts.add(r.end);
  }
  const pieces: Piece[] = [];
  let offset = 0;
  for (const token of tokens) {
    let from = 0;
    const edges = [...cuts]
      .map((c) => c - offset)
      .filter((c) => c > 0 && c < token.text.length)
      .sort((a, b) => a - b);
    for (const edge of [...edges, token.text.length]) {
      const start = offset + from;
      const text = token.text
        .slice(from, edge)
        .slice(0, Math.max(0, visible - start));
      if (text !== "") {
        pieces.push({
          text,
          cls: token.cls,
          mark: cover(marks, start),
          error: cover(errors, start),
        });
      }
      from = edge;
    }
    offset += token.text.length;
  }
  return pieces;
}

function renderPiece(p: Piece, key: number): ReactNode {
  return p.cls === null ? (
    p.text
  ) : (
    <span key={key} className={p.cls}>
      {p.text}
    </span>
  );
}

// 連続する同じ印の字句を 1 つの囲みにまとめる。マーカーが外、波線が内。
function renderPieces(
  pieces: readonly Piece[],
  marks: readonly Range[],
  errors: readonly Range[],
): ReactNode {
  const out: ReactNode[] = [];
  let i = 0;
  while (i < pieces.length) {
    const m = pieces[i].mark;
    let j = i;
    while (j < pieces.length && pieces[j].mark === m) j++;
    const inner: ReactNode[] = [];
    let k = i;
    while (k < j) {
      const e = pieces[k].error;
      let l = k;
      while (l < j && pieces[l].error === e) l++;
      const run = pieces.slice(k, l).map((p, n) => renderPiece(p, k + n));
      if (e < 0) {
        inner.push(...run);
      } else {
        const p = errors[e].progress;
        inner.push(
          <span
            key={`e${k}`}
            className="error"
            style={p < 1 ? ({ "--error-p": p } as CSSProperties) : undefined}
          >
            {run}
          </span>,
        );
      }
      k = l;
    }
    if (m < 0) {
      out.push(...inner);
    } else {
      const p = marks[m].progress;
      out.push(
        <span
          key={`m${i}`}
          className="mark"
          style={p < 1 ? ({ "--mark-p": p } as CSSProperties) : undefined}
        >
          {inner}
        </span>,
      );
    }
    i = j;
  }
  return out;
}

export function CodeBlock({
  block,
  scopes = [],
  marks = [],
  errors = [],
  lines: motion = {},
}: CodeBlockProps) {
  const blocks = useCodeBlocks();
  const data = blocks.find((b) => b.id === block);
  if (data === undefined) {
    throw new Error(`CodeBlock: code_blocks に id ${block} がない`);
  }

  const rows = parseLines(data);
  const isText = data.language === "text";
  const keys = lineKeys(
    data.language,
    rows.map((r) => r.number),
  );
  const rowText = new Map<number, string>();
  rows.forEach((r, i) => {
    const key = keys[i];
    if (key !== null) rowText.set(key, r.text);
  });

  checkLines(data.id, motion, keys);
  const placements = checkScopes(data.id, scopes, keys, data.diff === true);
  if (isText && (marks.length > 0 || errors.length > 0)) {
    throw new Error(
      `CodeBlock ${data.id}: language が text のコードブロックでは marks・errors を使えない`,
    );
  }
  const markRanges: Range[][] = rows.map(() => []);
  const errorRanges: Range[][] = rows.map(() => []);
  const place = (
    kind: "marks" | "errors",
    specs: readonly MarkSpec[],
    into: Range[][],
  ) => {
    for (const spec of specs) {
      const { range } = locate(data.id, kind, spec, rowText);
      const index = keys.indexOf(spec.line);
      if (range.progress > 0) into[index].push(range);
    }
  };
  place("marks", marks, markRanges);
  place("errors", errors, errorRanges);

  const numbers = displayNumbers(
    rows.map((r) => r.number),
    keys,
    motion,
  );
  const heights = rowHeights(keys, motion);

  const tokens: (readonly (Token | OutputToken)[])[] =
    data.language === "rust"
      ? splitLines(tokenizeRust(rows.map((l) => l.text).join("\n")))
      : isText
        ? tokenizeOutput(rows.map((l) => l.text))
        : rows.map((l) => [{ cls: null, text: l.text }]);

  return (
    <pre data-label={data.label}>
      <code
        className={`language-${data.language}`}
        data-diff={data.diff === true ? "" : undefined}
        data-scopes={placements.length > 0 ? "" : undefined}
      >
        {placements.map((s, i) => {
          const span = scopeSpan(s, keys, heights);
          const p = clamp01(s.progress ?? 1);
          return (
            <span
              key={`scope${i}`}
              className="scope"
              data-label={s.label}
              data-depth={s.depth}
              style={
                {
                  "--from": span.from,
                  "--to": span.to,
                  ...(p < 1 ? { "--scope-p": p } : {}),
                } as CSSProperties
              }
            />
          );
        })}
        {rows.map((line, i) => {
          const key = keys[i];
          const m = key === null ? undefined : motion[key];
          const open = m?.open === undefined ? 1 : clamp01(m.open);
          const typed = m?.typed === undefined ? 1 : clamp01(m.typed);
          const dim = m?.dim === undefined ? 0 : clamp01(m.dim);
          const visible =
            typed >= 1
              ? line.text.length
              : Math.floor(line.text.length * typed);
          const classes = ["line"];
          if (line.kind === "add") classes.push("is-add");
          if (line.kind === "del") classes.push("is-del");
          if (open < 1) classes.push("is-opening");
          const style: Record<string, number> = {};
          if (open < 1) style["--line-open"] = open;
          if (dim > 0) style["--line-dim"] = dim;
          const pieces = cutPieces(
            tokens[i],
            markRanges[i],
            errorRanges[i],
            visible,
          );
          return (
            <span
              key={i}
              className={classes.join(" ")}
              data-ln={numbers[i] === null ? "" : String(numbers[i])}
              style={
                Object.keys(style).length > 0
                  ? (style as CSSProperties)
                  : undefined
              }
            >
              {renderPieces(pieces, markRanges[i], errorRanges[i])}
              {typed > 0 && typed < 1 ? <span className="caret" /> : null}
            </span>
          );
        })}
      </code>
    </pre>
  );
}
