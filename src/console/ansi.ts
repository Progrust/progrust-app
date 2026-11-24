// 出力の ANSI エスケープの読み方（documents/spec/screens/exercise-screen.md「コンパイラの出力は ANSI の色を反映して表示する」、
// ADR 0226）。SGR の色と太字だけを読み、ほかのエスケープは表示から除く。
// 出力は断片で届き、エスケープが断片の境目で切れることがあるので、読みかけのエスケープと
// いまの色を状態として持ち、次の断片に持ち越す。

// 色は、デザインシステムの ansi-* のトークンがある 6 色だけにする。
export type AnsiColor =
  "red" | "green" | "yellow" | "blue" | "magenta" | "cyan";

export type AnsiStyle = {
  // null は、出力の種類の既定の色。
  color: AnsiColor | null;
  bold: boolean;
};

export type AnsiState = {
  style: AnsiStyle;
  // 断片の終わりで切れた、読みかけのエスケープ。
  pending: string;
};

// 色の付いた一続きの文字列。
export type StyledText = {
  style: AnsiStyle;
  text: string;
};

export const PLAIN_STYLE: AnsiStyle = { color: null, bold: false };

export const INITIAL_ANSI_STATE: AnsiState = {
  style: PLAIN_STYLE,
  pending: "",
};

const ESC = "\x1b";
const BEL = "\x07";

// 色の番号（0〜7）の色。黒（0）と白（7）は、テーマの文字の色と見分けにくいので、既定の色で表示する。
const BASIC_COLORS: readonly (AnsiColor | null)[] = [
  null,
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  null,
];

// 読みかけのまま持ち越すエスケープの長さの上限。これを超えたら、エスケープとみなさない。
// 正しい SGR はずっと短い。OSC（リンクなど）は長くなりうるので、上限を大きめにする。
const MAX_PENDING = 4096;

export function sameStyle(a: AnsiStyle, b: AnsiStyle): boolean {
  return a.color === b.color && a.bold === b.bold;
}

// 256 色の番号を、6 色に寄せる。0〜15 は、明るい色（8〜15）を同じ色の番号（0〜7）に寄せる。
// 16〜255 は 6 色に当てられないので、既定の色にする。
function indexedColor(index: number): AnsiColor | null {
  if (index >= 0 && index < 8) {
    return BASIC_COLORS[index];
  }
  if (index >= 8 && index < 16) {
    return BASIC_COLORS[index - 8];
  }
  return null;
}

// SGR の引数（`1;38;5;9` など）を読み、新しい色と太字を返す。読まない指定は無視する。
export function applySgr(style: AnsiStyle, params: string): AnsiStyle {
  let { color, bold } = style;
  const items = params.split(";");
  for (let i = 0; i < items.length; i++) {
    // `38:5:9` のように、コロンで区切った書き方もある。
    const parts = items[i].split(":");
    const code = parts[0] === "" ? 0 : Number(parts[0]);
    if (code === 38 || code === 48 || code === 58) {
      // 文字・背景・下線の色の拡張の指定。後ろの引数も一緒に読み、背景と下線の色は無視する。
      let kind: string | undefined;
      let index: string | undefined;
      if (parts.length > 1) {
        kind = parts[1];
        index = parts[2];
      } else {
        kind = items[i + 1];
        if (kind === "5") {
          index = items[i + 2];
          i += 2;
        } else if (kind === "2") {
          i += 4;
        } else {
          i += 1;
        }
      }
      if (code === 38) {
        color = kind === "5" ? indexedColor(Number(index)) : null;
      }
    } else if (code === 0) {
      color = null;
      bold = false;
    } else if (code === 1) {
      bold = true;
    } else if (code === 22) {
      bold = false;
    } else if (code >= 30 && code <= 37) {
      color = BASIC_COLORS[code - 30];
    } else if (code === 39) {
      color = null;
    } else if (code >= 90 && code <= 97) {
      color = BASIC_COLORS[code - 90];
    }
  }
  return { color, bold };
}

type Escape =
  // エスケープの終わりの位置（次に読む位置）と、SGR ならその引数。
  | { kind: "done"; end: number; sgr: string | null }
  // 断片の終わりで切れている。
  | { kind: "incomplete" };

const isParam = (c: number) => c >= 0x30 && c <= 0x3f;
const isIntermediate = (c: number) => c >= 0x20 && c <= 0x2f;
const isFinal = (c: number) => c >= 0x40 && c <= 0x7e;

// start の ESC から始まるエスケープを読む。
function readEscape(text: string, start: number): Escape {
  if (start + 1 >= text.length) {
    return { kind: "incomplete" };
  }
  const next = text[start + 1];
  if (next === "[") {
    // CSI: ESC [ 引数 中間 終わりの文字。
    let i = start + 2;
    while (i < text.length && isParam(text.charCodeAt(i))) i++;
    const paramsEnd = i;
    while (i < text.length && isIntermediate(text.charCodeAt(i))) i++;
    if (i >= text.length) {
      return { kind: "incomplete" };
    }
    if (!isFinal(text.charCodeAt(i))) {
      // 壊れたエスケープ。そこまでを捨て、続きは文字として読む。
      return { kind: "done", end: i, sgr: null };
    }
    const isSgr = text[i] === "m" && paramsEnd === i;
    return {
      kind: "done",
      end: i + 1,
      sgr: isSgr ? text.slice(start + 2, paramsEnd) : null,
    };
  }
  if (
    next === "]" ||
    next === "P" ||
    next === "X" ||
    next === "^" ||
    next === "_"
  ) {
    // OSC・DCS など: ST（ESC \）か BEL で終わる。
    for (let i = start + 2; i < text.length; i++) {
      if (text[i] === BEL) {
        return { kind: "done", end: i + 1, sgr: null };
      }
      if (text[i] === ESC) {
        if (i + 1 >= text.length) {
          return { kind: "incomplete" };
        }
        // ST でない ESC は、次のエスケープの始まりとして読む（壊れた OSC はそこまでを捨てる）。
        return {
          kind: "done",
          end: text[i + 1] === "\\" ? i + 2 : i,
          sgr: null,
        };
      }
    }
    return { kind: "incomplete" };
  }
  const code = text.charCodeAt(start + 1);
  if (isIntermediate(code)) {
    // ESC 中間 終わりの文字（文字集合の指定など）。
    let i = start + 1;
    while (i < text.length && isIntermediate(text.charCodeAt(i))) i++;
    if (i >= text.length) {
      return { kind: "incomplete" };
    }
    const c = text.charCodeAt(i);
    return { kind: "done", end: c >= 0x30 && c <= 0x7e ? i + 1 : i, sgr: null };
  }
  if (code >= 0x30 && code <= 0x7e) {
    // 2 文字のエスケープ。
    return { kind: "done", end: start + 2, sgr: null };
  }
  // ESC だけを捨てる。
  return { kind: "done", end: start + 1, sgr: null };
}

// 断片を読み、色の付いた文字列の並びと、次の断片に持ち越す状態を返す。
export function parseAnsi(
  state: AnsiState,
  input: string,
): { state: AnsiState; pieces: StyledText[] } {
  const text = state.pending + input;
  const pieces: StyledText[] = [];
  let style = state.style;
  const push = (piece: string) => {
    if (piece === "") {
      return;
    }
    const last = pieces[pieces.length - 1];
    if (last !== undefined && sameStyle(last.style, style)) {
      last.text += piece;
    } else {
      pieces.push({ style, text: piece });
    }
  };

  let pos = 0;
  let pending = "";
  while (pos < text.length) {
    const esc = text.indexOf(ESC, pos);
    if (esc < 0) {
      push(text.slice(pos));
      break;
    }
    push(text.slice(pos, esc));
    const escape = readEscape(text, esc);
    if (escape.kind === "incomplete") {
      if (text.length - esc <= MAX_PENDING) {
        pending = text.slice(esc);
        break;
      }
      // 長すぎるものはエスケープとみなさず、ESC だけを捨てて、続きを文字として読む。
      pos = esc + 1;
      continue;
    }
    if (escape.sgr !== null) {
      style = applySgr(style, escape.sgr);
    }
    pos = escape.end;
  }
  return { state: { style, pending }, pieces };
}
