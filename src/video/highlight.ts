// コードの字句の色分け（design-system/progrust-video/README.md「コードの色」）。
// ライブラリを使わず、Rust の字句だけを自作の走査で分ける（ADR 0188）。
// 実行時に外部から読み込まない。

// 字句の区分（クラス名。null は色を付けない）。
export type TokenClass =
  | "tk-k"
  | "tk-s"
  | "tk-m"
  | "tk-a"
  | "tk-t"
  | "tk-n"
  | "tk-l"
  | "tk-f"
  | "tk-c"
  | null;

export type Token = { cls: TokenClass; text: string };

const KEYWORDS = new Set([
  "as",
  "async",
  "await",
  "break",
  "const",
  "continue",
  "crate",
  "dyn",
  "else",
  "enum",
  "extern",
  "fn",
  "for",
  "if",
  "impl",
  "in",
  "let",
  "loop",
  "match",
  "mod",
  "move",
  "mut",
  "pub",
  "ref",
  "return",
  "self",
  "static",
  "struct",
  "super",
  "trait",
  "type",
  "unsafe",
  "use",
  "where",
  "while",
]);

const PRIMITIVES = new Set([
  "bool",
  "char",
  "f32",
  "f64",
  "i8",
  "i16",
  "i32",
  "i64",
  "i128",
  "isize",
  "str",
  "u8",
  "u16",
  "u32",
  "u64",
  "u128",
  "usize",
]);

const isIdentStart = (c: string | undefined) =>
  c !== undefined && /[\p{L}_]/u.test(c);
const isIdentPart = (c: string | undefined) =>
  c !== undefined && /[\p{L}\p{N}_]/u.test(c);
const isDigit = (c: string | undefined) => c !== undefined && /[0-9]/.test(c);

// 1 つの文字列を、字句に分ける。字句をつなげると、元の文字列になる。
export function tokenizeRust(src: string): Token[] {
  const tokens: Token[] = [];
  let plain = "";
  const flush = () => {
    if (plain !== "") {
      tokens.push({ cls: null, text: plain });
      plain = "";
    }
  };
  const push = (cls: TokenClass, text: string) => {
    flush();
    tokens.push({ cls, text });
  };

  let i = 0;
  while (i < src.length) {
    const c = src[i];
    const next = src[i + 1];

    if (c === "/" && next === "/") {
      let j = i;
      while (j < src.length && src[j] !== "\n") j++;
      push("tk-c", src.slice(i, j));
      i = j;
      continue;
    }
    if (c === "/" && next === "*") {
      let j = i + 2;
      let depth = 1;
      while (j < src.length && depth > 0) {
        if (src[j] === "/" && src[j + 1] === "*") {
          depth++;
          j += 2;
        } else if (src[j] === "*" && src[j + 1] === "/") {
          depth--;
          j += 2;
        } else {
          j++;
        }
      }
      push("tk-c", src.slice(i, j));
      i = j;
      continue;
    }

    // 生文字列 r"…"・r#"…"#（b・c の接頭辞つきも）
    const raw = /^(?:br|cr|r)(#*)"/.exec(src.slice(i, i + 16));
    if (raw !== null && !isIdentPart(src[i - 1])) {
      const closing = `"${raw[1]}`;
      const end = src.indexOf(closing, i + raw[0].length);
      const j = end < 0 ? src.length : end + closing.length;
      push("tk-s", src.slice(i, j));
      i = j;
      continue;
    }

    if (
      c === '"' ||
      ((c === "b" || c === "c") && next === '"' && !isIdentPart(src[i - 1]))
    ) {
      let j = c === '"' ? i + 1 : i + 2;
      while (j < src.length && src[j] !== '"') {
        j += src[j] === "\\" ? 2 : 1;
      }
      j = Math.min(j + 1, src.length);
      push("tk-s", src.slice(i, j));
      i = j;
      continue;
    }

    if (c === "'") {
      // 文字（'a'・'\n'）か、ライフタイム（'a）か。
      const ch =
        /^'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'])'/su.exec(
          src.slice(i, i + 16),
        );
      if (ch !== null) {
        push("tk-s", ch[0]);
        i += ch[0].length;
        continue;
      }
      if (isIdentStart(next)) {
        let j = i + 1;
        while (isIdentPart(src[j])) j++;
        push("tk-l", src.slice(i, j));
        i = j;
        continue;
      }
    }

    if (c === "#" && (next === "[" || (next === "!" && src[i + 2] === "["))) {
      let j = src.indexOf("[", i) + 1;
      let depth = 1;
      while (j < src.length && depth > 0) {
        if (src[j] === "[") depth++;
        else if (src[j] === "]") depth--;
        j++;
      }
      push("tk-a", src.slice(i, j));
      i = j;
      continue;
    }

    if (isDigit(c) && !isIdentPart(src[i - 1])) {
      let j = i + 1;
      while (j < src.length) {
        if (isIdentPart(src[j])) {
          j++;
        } else if (src[j] === "." && isDigit(src[j + 1])) {
          j++;
        } else {
          break;
        }
      }
      push("tk-n", src.slice(i, j));
      i = j;
      continue;
    }

    if (isIdentStart(c)) {
      let j = i + 1;
      while (isIdentPart(src[j])) j++;
      const word = src.slice(i, j);
      if (src[j] === "!" && src[j + 1] !== "=") {
        push("tk-m", `${word}!`);
        i = j + 1;
        continue;
      }
      if (word === "true" || word === "false") {
        push("tk-n", word);
      } else if (KEYWORDS.has(word)) {
        push("tk-k", word);
      } else if (PRIMITIVES.has(word) || /^\p{Lu}/u.test(word)) {
        push("tk-t", word);
      } else if (
        src[j] === "(" ||
        (src[j] === ":" && src.startsWith("::<", j))
      ) {
        push("tk-f", word);
      } else {
        plain += word;
      }
      i = j;
      continue;
    }

    plain += c;
    i++;
  }
  flush();
  return tokens;
}

// 字句の列を、行ごとに分ける（複数行にまたがる字句は、行ごとに切る）。
export function splitLines(tokens: Token[]): Token[][] {
  const lines: Token[][] = [[]];
  for (const token of tokens) {
    const parts = token.text.split("\n");
    parts.forEach((part, index) => {
      if (index > 0) lines.push([]);
      if (part !== "")
        lines[lines.length - 1].push({ cls: token.cls, text: part });
    });
  }
  return lines;
}

// ---- rustc の出力（language が text）の色付け ----

export type OutputClass =
  | "out-error"
  | "out-warning"
  | "out-note"
  | "out-loc"
  | "out-caret"
  | "out-help"
  | null;

export type OutputToken = { cls: OutputClass; text: string };

// 出力を行ごとに解析して、行ごとの字句にする。
export function tokenizeOutput(lines: readonly string[]): OutputToken[][] {
  // 直前の見出しが warning か（`^^^` の色を決める）。
  let inWarning = false;
  return lines.map((line): OutputToken[] => {
    const heading = /^(\s*)(error|warning)(\[[A-Za-z0-9]+\])?(:)(.*)$/.exec(
      line,
    );
    if (heading !== null) {
      const [, indent, kind, code = "", colon, rest] = heading;
      inWarning = kind === "warning";
      return [
        { cls: null, text: indent },
        { cls: inWarning ? "out-warning" : "out-error", text: kind + code },
        { cls: "out-note", text: colon + rest },
      ].filter((t) => t.text !== "") as OutputToken[];
    }
    if (/^\s*-->/.test(line)) {
      return [{ cls: "out-loc", text: line }];
    }
    const note = /^(\s*(?:=\s+)?)(note|help)(:)(.*)$/.exec(line);
    if (note !== null) {
      const [, lead, kind, colon, rest] = note;
      return [
        { cls: "out-loc", text: lead },
        { cls: kind === "help" ? "out-help" : "out-note", text: kind + colon },
        { cls: null, text: rest },
      ].filter((t) => t.text !== "") as OutputToken[];
    }
    const gutter = /^(\s*)(\d*)(\s*\|)(.*)$/.exec(line);
    if (gutter !== null) {
      const [, indent, num, bar, rest] = gutter;
      const head: OutputToken[] = [
        { cls: null, text: indent },
        { cls: "out-loc", text: num + bar },
      ];
      const marker = /^(\s*)(\^+|-+)(.*)$/.exec(rest);
      if (num === "" && marker !== null) {
        const cls: OutputClass =
          marker[2][0] === "^"
            ? inWarning
              ? "out-warning"
              : "out-caret"
            : "out-loc";
        head.push(
          { cls: null, text: marker[1] },
          { cls, text: marker[2] + marker[3] },
        );
      } else {
        head.push({ cls: null, text: rest });
      }
      return head.filter((t) => t.text !== "");
    }
    return line === "" ? [] : [{ cls: null, text: line }];
  });
}
