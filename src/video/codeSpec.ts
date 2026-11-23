// CodeBlock の指定（scopes・marks・errors・lines）の型と、制約の検査、行番号と枠の位置の計算
// （documents/spec/content-format/video/code-display.md「`CodeBlock` の指定」）。
// 制約に違反する指定は、描画時にエラーを投げる。
import { clamp01 } from "./motion";

export type ScopeSpec = {
  label: string;
  from: number;
  to: number;
  // 現れ方の進み具合（0〜1。省略時は 1）。
  progress?: number;
};

export type MarkSpec = {
  line: number;
  text: string;
  // その行に同じ文字列が複数あるときの、何個目か（1 始まり。省略時は 1）。
  nth?: number;
  progress?: number;
};

export type LineMotion = {
  // 行の差し込み。1 未満の行は、まだ差し込み終わっていない。
  open?: number;
  // 入力の進み具合。
  typed?: number;
  // 淡くする度合い。
  dim?: number;
};

// 行番号ごとの動き。text では、1 から数えた行の位置で書く。
export type LinesSpec = Readonly<Record<number, LineMotion>>;

export const MAX_SCOPES = 3;
export const MAX_SCOPE_DEPTH = 2;

// 行ごとの、指定に書く行番号。指定に書けない行（Diff の削除行）は null。
// text では、1 から数えた行の位置。
export function lineKeys(
  language: string,
  numbers: readonly (number | null)[],
): (number | null)[] {
  return language === "text" ? numbers.map((_, i) => i + 1) : [...numbers];
}

function rangeOf(keys: readonly (number | null)[]): [number, number] {
  const found = keys.filter((k): k is number => k !== null);
  return [Math.min(...found), Math.max(...found)];
}

export type ScopePlacement = ScopeSpec & { depth: number };

// scopes の制約を検査し、入れ子の深さを付けて返す（指定の順のまま）。
export function checkScopes(
  id: string,
  scopes: readonly ScopeSpec[],
  keys: readonly (number | null)[],
  diff: boolean,
): ScopePlacement[] {
  if (scopes.length === 0) return [];
  if (diff) {
    throw new Error(
      `CodeBlock ${id}: diff のコードブロックでは scopes を使えない`,
    );
  }
  if (scopes.length > MAX_SCOPES) {
    throw new Error(
      `CodeBlock ${id}: scopes は ${MAX_SCOPES} つまで（${scopes.length} 個指定）`,
    );
  }
  const [min, max] = rangeOf(keys);
  for (const s of scopes) {
    if (!Number.isInteger(s.from) || !Number.isInteger(s.to) || s.from > s.to) {
      throw new Error(
        `CodeBlock ${id}: scopes ${s.label} の範囲 ${s.from}〜${s.to} が正しくない`,
      );
    }
    if (s.from < min || s.to > max) {
      throw new Error(
        `CodeBlock ${id}: scopes ${s.label} の範囲 ${s.from}〜${s.to} が、code_blocks の行番号 ${min}〜${max} の外にある`,
      );
    }
  }
  for (let i = 0; i < scopes.length; i++) {
    for (let j = i + 1; j < scopes.length; j++) {
      const a = scopes[i];
      const b = scopes[j];
      const disjoint = a.to < b.from || b.to < a.from;
      const aInB = b.from <= a.from && a.to <= b.to;
      const bInA = a.from <= b.from && b.to <= a.to;
      if (!disjoint && !aInB && !bInA) {
        throw new Error(
          `CodeBlock ${id}: scopes ${a.label} と ${b.label} の範囲が一部だけ重なっている`,
        );
      }
    }
  }
  // 深さ = 自分を含む範囲の数。同じ範囲どうしは、先に書いたほうを外側とする。
  const placements = scopes.map((s, i) => {
    let depth = 0;
    scopes.forEach((o, j) => {
      if (i === j) return;
      const contains = o.from <= s.from && s.to <= o.to;
      const equal = o.from === s.from && o.to === s.to;
      if (contains && (!equal || j < i)) depth++;
    });
    return { ...s, depth };
  });
  const deepest = Math.max(...placements.map((p) => p.depth)) + 1;
  if (deepest > MAX_SCOPE_DEPTH) {
    throw new Error(
      `CodeBlock ${id}: scopes の入れ子は ${MAX_SCOPE_DEPTH} 段まで（${deepest} 段ある）`,
    );
  }
  return placements;
}

export type Range = { start: number; end: number; progress: number };

// marks・errors の 1 件を、行の中の文字の範囲にする。
export function locate(
  id: string,
  kind: "marks" | "errors",
  spec: MarkSpec,
  rows: ReadonlyMap<number, string>,
): { row: number; range: Range } {
  const text = rows.get(spec.line);
  if (text === undefined) {
    throw new Error(
      `CodeBlock ${id}: ${kind} の行 ${spec.line} が、code_blocks の行番号の外にある`,
    );
  }
  const nth = spec.nth ?? 1;
  if (!Number.isInteger(nth) || nth < 1) {
    throw new Error(`CodeBlock ${id}: ${kind} の nth は 1 以上の整数にする`);
  }
  if (spec.text === "") {
    throw new Error(`CodeBlock ${id}: ${kind} の text が空`);
  }
  let at = -1;
  for (let k = 0; k < nth; k++) {
    at = text.indexOf(spec.text, at < 0 ? 0 : at + spec.text.length);
    if (at < 0) {
      throw new Error(
        `CodeBlock ${id}: ${kind} の行 ${spec.line} に「${spec.text}」が ${nth} 個ない`,
      );
    }
  }
  return {
    row: spec.line,
    range: {
      start: at,
      end: at + spec.text.length,
      progress: clamp01(spec.progress ?? 1),
    },
  };
}

// lines の行番号が、code_blocks の行番号の中にあることを検査する。
export function checkLines(
  id: string,
  lines: LinesSpec,
  keys: readonly (number | null)[],
): void {
  const valid = new Set(keys.filter((k): k is number => k !== null));
  for (const key of Object.keys(lines)) {
    if (!valid.has(Number(key))) {
      throw new Error(
        `CodeBlock ${id}: lines の行 ${key} が、code_blocks の行番号の外にある`,
      );
    }
  }
}

// 行を差し込む動きの途中の行番号。まだ差し込み終わっていない行（open < 1）は数えず、
// その行自身には番号を出さない。差し込み終わると、code_blocks の行番号と同じになる。
export function displayNumbers(
  numbers: readonly (number | null)[],
  keys: readonly (number | null)[],
  lines: LinesSpec,
): (number | null)[] {
  const pending = (key: number | null): boolean => {
    if (key === null) return false;
    const open = lines[key]?.open;
    return open !== undefined && clamp01(open) < 1;
  };
  let skipped = 0;
  return numbers.map((n, i) => {
    if (pending(keys[i])) {
      skipped++;
      return null;
    }
    return n === null ? null : n - skipped;
  });
}

// 行ごとの高さ（行の差し込みの途中は 0〜1、ほかは 1）。
export function rowHeights(
  keys: readonly (number | null)[],
  lines: LinesSpec,
): number[] {
  return keys.map((k) => {
    const open = k === null ? undefined : lines[k]?.open;
    return open === undefined ? 1 : clamp01(open);
  });
}

// 範囲の枠の位置（載せた何行目か。差し込みの途中は、開いている分を足した小数）。
export function scopeSpan(
  scope: ScopeSpec,
  keys: readonly (number | null)[],
  heights: readonly number[],
): { from: number; to: number } {
  const first = keys.indexOf(scope.from);
  const last = keys.indexOf(scope.to);
  let before = 0;
  for (let i = 0; i < first; i++) before += heights[i];
  let through = before;
  for (let i = first; i <= last; i++) through += heights[i];
  const round = (x: number) => Math.round(x * 1000) / 1000;
  return { from: round(before + 1), to: round(through) };
}
