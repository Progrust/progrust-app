// 検査「訳語の表」（documents/spec/content-guidelines/check-tool.md）。
// content/terms.json が訳語の表の形式（documents/spec/content-format/terms-json.md）に合い、
// term に重複がないことを確かめる。
import { join } from "node:path";
import type { Check, CheckOptions, Finding } from "./types.ts";
import { isRecord, readJsonFile } from "./video-files.ts";

const NAME = "訳語の表";
const FILE = "terms.json";

export function checkTerms({ contentDir }: CheckOptions): Finding[] {
  const out: Finding[] = [];
  const error = (message: string) =>
    out.push({ severity: "error", check: NAME, file: FILE, message });

  const result = readJsonFile(join(contentDir, FILE));
  if (result.state === "missing") {
    error(`${FILE} が無い`);
    return out;
  }
  if (result.state === "invalid") {
    error(`${FILE} を読めない: ${result.message}`);
    return out;
  }
  if (!Array.isArray(result.value)) {
    error(`${FILE} が配列でない`);
    return out;
  }

  const seen = new Set<string>();
  result.value.forEach((item: unknown, i: number) => {
    const at = `${i + 1} 番目の項目`;
    if (!isRecord(item)) {
      error(`${at}がオブジェクトでない`);
      return;
    }
    for (const key of Object.keys(item)) {
      if (!["term", "code", "avoid", "note"].includes(key)) {
        error(
          `${at}に、決まっていないフィールド ${JSON.stringify(key)} がある`,
        );
      }
    }
    if (typeof item.term !== "string" || item.term === "") {
      error(`${at}の term が空でない文字列でない`);
    } else {
      if (seen.has(item.term)) {
        error(`term ${JSON.stringify(item.term)} が重複している`);
      }
      seen.add(item.term);
    }
    for (const key of ["code", "note"]) {
      if (key in item && typeof item[key] !== "string") {
        error(`${at}の ${key} が文字列でない`);
      }
    }
    const avoid = item.avoid;
    if (
      !Array.isArray(avoid) ||
      !avoid.every((a: unknown) => typeof a === "string")
    ) {
      error(`${at}の avoid が文字列の配列でない（無いときは空の配列にする）`);
    }
  });
  return out;
}

export const termsCheck: Check = { name: NAME, run: checkTerms };
