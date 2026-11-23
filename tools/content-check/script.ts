// 検査「台本」（documents/spec/content-guidelines/check-tool.md）。
// script.json の形式（documents/spec/content-format/video/script-json.md）、
// code_blocks の行番号と Diff の決まり（documents/spec/content-format/video/code-display.md）、
// index.tsx のキーと台本の scenes の一致（documents/spec/content-format/video/scenes.md）を確かめる。
// 準備中のレッスンと章末復習レッスンは検査しない。ファイルが無いことは「構成」が報告する。
import { readFileSync } from "node:fs";
import ts from "typescript";
import {
  codePointLength,
  exists,
  isRecord,
  readJsonFile,
  readVideoLessons,
  videoPath,
  videoRel,
} from "./video-files.ts";
import type { LessonEntry } from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

const NAME = "台本";
const ID_PATTERN = /^[a-z][a-z0-9-]*$/;
const SAY_MAX = 140;
// 表示する最後の行番号の上限。
const LINE_MAX = 999;

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

// 台本の形式の違反を、メッセージで返す。台本の形が合っていれば空。
export function validateScript(script: unknown): string[] {
  const out: string[] = [];
  if (!isRecord(script)) return ["台本がオブジェクトでない"];

  if (script.ai_note !== undefined && typeof script.ai_note !== "string") {
    out.push("ai_note が文字列でない");
  }

  const checkIds = (kind: string, items: unknown[]): Set<string> => {
    const seen = new Set<string>();
    for (const item of items) {
      const id = isRecord(item) ? item.id : undefined;
      if (typeof id !== "string" || !ID_PATTERN.test(id)) {
        out.push(
          `${kind}の id が、英小文字で始まり英小文字・数字・- からなる文字列でない`,
        );
      } else if (seen.has(id)) {
        out.push(`${kind}の id ${id} が重複している`);
      } else {
        seen.add(id);
      }
    }
    return seen;
  };

  // code_blocks
  let codeBlocks: unknown[] = [];
  if (script.code_blocks !== undefined) {
    if (Array.isArray(script.code_blocks)) codeBlocks = script.code_blocks;
    else out.push("code_blocks が配列でない");
  }
  checkIds("code_blocks", codeBlocks);
  for (const block of codeBlocks) {
    if (!isRecord(block)) {
      out.push("code_blocks の要素がオブジェクトでない");
      continue;
    }
    out.push(...validateCodeBlock(block));
  }

  // scenes
  let scenes: unknown[] = [];
  if (Array.isArray(script.scenes)) scenes = script.scenes;
  else out.push("scenes が配列でない");
  if (Array.isArray(script.scenes) && scenes.length === 0) {
    out.push("scenes が 1 件もない");
  }
  const sceneIds = checkIds("scenes", scenes);
  for (const scene of scenes) {
    if (!isRecord(scene)) {
      out.push("scenes の要素がオブジェクトでない");
      continue;
    }
    const label =
      typeof scene.id === "string" ? `シーン ${scene.id}` : "シーン";
    if (typeof scene.name !== "string")
      out.push(`${label}の name が文字列でない`);
    for (const key of ["lead", "tail"]) {
      const value = scene[key];
      if (value !== undefined && !(isNumber(value) && value >= 0)) {
        out.push(`${label}の ${key} が 0 以上の数でない`);
      }
    }
  }

  // sentences
  let sentences: unknown[] = [];
  if (Array.isArray(script.sentences)) sentences = script.sentences;
  else out.push("sentences が配列でない");
  checkIds("sentences", sentences);

  const sceneOrder = scenes.flatMap((s) =>
    isRecord(s) && typeof s.id === "string" ? [s.id] : [],
  );
  const runs: string[] = [];
  const usedScenes = new Set<string>();
  sentences.forEach((sentence, i) => {
    if (!isRecord(sentence)) {
      out.push("sentences の要素がオブジェクトでない");
      return;
    }
    const label =
      typeof sentence.id === "string"
        ? `文 ${sentence.id}`
        : `${i + 1} 件目の文`;
    if (typeof sentence.scene !== "string" || !sceneIds.has(sentence.scene)) {
      out.push(`${label}の scene が、scenes の id にない`);
    } else {
      usedScenes.add(sentence.scene);
      if (runs[runs.length - 1] !== sentence.scene) runs.push(sentence.scene);
    }
    for (const key of ["text", "say"]) {
      const value = sentence[key];
      if (typeof value !== "string" || value.trim() === "") {
        out.push(`${label}の ${key} が空か、文字列でない`);
      }
    }
    if (
      typeof sentence.say === "string" &&
      codePointLength(sentence.say) > SAY_MAX
    ) {
      out.push(
        `${label}の say が ${codePointLength(sentence.say)} 字で、${SAY_MAX} 字を超えている`,
      );
    }
    const pause = sentence.pause_after;
    if (pause !== undefined && !(isNumber(pause) && pause >= 0)) {
      out.push(`${label}の pause_after が 0 以上の数でない`);
    }
    // シーンの最後の文には、pause_after を書けない。
    const next = sentences[i + 1];
    const isLast = !isRecord(next) || next.scene !== sentence.scene;
    if (pause !== undefined && isLast) {
      out.push(`${label}はシーンの最後の文なので、pause_after を書けない`);
    }
  });

  for (const id of sceneOrder) {
    if (!usedScenes.has(id)) out.push(`シーン ${id} に文がない`);
  }
  const seenRuns = new Set<string>();
  for (const id of runs) {
    if (seenRuns.has(id)) {
      out.push(`シーン ${id} の文が続けて並んでいない`);
      break;
    }
    seenRuns.add(id);
  }
  const firstOrder = [...seenRuns];
  const expected = sceneOrder.filter((id) => seenRuns.has(id));
  if (firstOrder.join("\n") !== expected.join("\n")) {
    out.push("シーンが現れる順序が、scenes の順序と違う");
  }
  return out;
}

function validateCodeBlock(block: Record<string, unknown>): string[] {
  const out: string[] = [];
  const label =
    typeof block.id === "string" ? `code_blocks の ${block.id}` : "code_blocks";
  if (typeof block.language !== "string" || block.language === "") {
    out.push(`${label}の language が文字列でない`);
  }
  if (block.label !== undefined && typeof block.label !== "string") {
    out.push(`${label}の label が文字列でない`);
  }
  if (typeof block.code !== "string") {
    out.push(`${label}の code が文字列でない`);
    return out;
  }
  if (block.diff !== undefined && typeof block.diff !== "boolean") {
    out.push(`${label}の diff が真偽値でない`);
  }
  const start = block.start_line;
  const startValid =
    start === undefined || (Number.isInteger(start) && (start as number) >= 1);
  if (!startValid) out.push(`${label}の start_line が 1 以上の整数でない`);

  const lines = block.code.split("\n");
  const isDiff = block.diff === true;
  let shown = lines.length;
  if (isDiff) {
    const bad = lines.findIndex((line) => line !== "" && !/^[-+ ]/.test(line));
    if (bad >= 0) {
      out.push(
        `${label}は Diff だが、${bad + 1} 行目の先頭が +・-・半角空白のどれでもない`,
      );
    }
    // 削除行は行番号を持たない。
    shown = lines.filter((line) => !line.startsWith("-")).length;
  }
  // language が text のコードブロックは、行番号を表示しない。
  if (startValid && block.language !== "text") {
    const last = (typeof start === "number" ? start : 1) + shown - 1;
    if (last > LINE_MAX) {
      out.push(`${label}の最後の行番号が ${last} で、${LINE_MAX} を超えている`);
    }
  }
  return out;
}

// index.tsx の default export のキー。読めないときは理由の文字列。
export function readSceneKeys(source: string): string[] | string {
  const file = ts.createSourceFile(
    "index.tsx",
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  const unwrap = (node: ts.Expression): ts.Expression => {
    let current = node;
    while (
      ts.isParenthesizedExpression(current) ||
      ts.isAsExpression(current) ||
      ts.isSatisfiesExpression(current)
    ) {
      current = current.expression;
    }
    return current;
  };

  let exported: ts.Expression | undefined;
  for (const statement of file.statements) {
    if (ts.isExportAssignment(statement) && !statement.isExportEquals) {
      exported = unwrap(statement.expression);
    }
  }
  if (exported === undefined) return "default export がない";

  // `export default scenes;` のように、同じファイルの変数を指すときは、その初期値を見る。
  if (ts.isIdentifier(exported)) {
    const name = exported.text;
    let init: ts.Expression | undefined;
    for (const statement of file.statements) {
      if (!ts.isVariableStatement(statement)) continue;
      for (const decl of statement.declarationList.declarations) {
        if (
          ts.isIdentifier(decl.name) &&
          decl.name.text === name &&
          decl.initializer
        ) {
          init = unwrap(decl.initializer);
        }
      }
    }
    if (init === undefined) return `default export の ${name} の中身を読めない`;
    exported = init;
  }
  if (!ts.isObjectLiteralExpression(exported)) {
    return "default export がオブジェクトのリテラルでない";
  }

  const keys: string[] = [];
  for (const prop of exported.properties) {
    if (ts.isSpreadAssignment(prop))
      return "default export に展開（...）がある";
    const name = prop.name;
    if (
      name !== undefined &&
      (ts.isIdentifier(name) ||
        ts.isStringLiteral(name) ||
        ts.isNoSubstitutionTemplateLiteral(name))
    ) {
      keys.push(name.text);
    } else {
      return "default export のキーが、名前か文字列でない";
    }
  }
  return keys;
}

function checkLesson(entry: LessonEntry): Finding[] {
  const out: Finding[] = [];
  const error = (file: string, message: string) =>
    out.push({
      severity: "error",
      check: NAME,
      file: videoRel(entry, file),
      lesson: entry.id,
      message,
    });

  const script = readJsonFile(videoPath(entry, "script.json"));
  if (script.state === "invalid") {
    error(
      "script.json",
      `script.json を JSON として読めない（${script.message}）`,
    );
    return out;
  }
  if (script.state === "ok") {
    for (const message of validateScript(script.value)) {
      error("script.json", message);
    }
  }

  const indexPath = videoPath(entry, "index.tsx");
  if (script.state === "ok" && exists(indexPath)) {
    const keys = readSceneKeys(readFileSync(indexPath, "utf8"));
    if (typeof keys === "string") {
      error("index.tsx", keys);
    } else if (isRecord(script.value) && Array.isArray(script.value.scenes)) {
      const ids = script.value.scenes.flatMap((s: unknown) =>
        isRecord(s) && typeof s.id === "string" ? [s.id] : [],
      );
      for (const id of ids) {
        if (!keys.includes(id)) {
          error("index.tsx", `台本のシーン ${id} が、index.tsx のキーにない`);
        }
      }
      const seen = new Set<string>();
      for (const key of keys) {
        if (seen.has(key)) {
          error("index.tsx", `index.tsx のキー ${key} が重複している`);
        }
        seen.add(key);
        if (!ids.includes(key)) {
          error(
            "index.tsx",
            `index.tsx のキー ${key} が、台本の scenes にない`,
          );
        }
      }
    }
  }
  return out;
}

export function checkScript({ contentDir }: CheckOptions): Finding[] {
  return readVideoLessons(contentDir).flatMap(checkLesson);
}

export const scriptCheck: Check = { name: NAME, run: checkScript };
