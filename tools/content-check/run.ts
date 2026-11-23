// 教材チェックツールの本体。検査の項目は checks に足していく。
import { curriculumCheck } from "./curriculum.ts";
import { definitionCheck } from "./definition.ts";
import { elementsCheck } from "./elements.ts";
import { structureCheck } from "./structure.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

export const checks: Check[] = [
  curriculumCheck,
  structureCheck,
  definitionCheck,
  elementsCheck,
];

export function runChecks(
  options: CheckOptions,
  list: Check[] = checks,
): Finding[] {
  return list.flatMap((check) => check.run(options));
}

export function formatFinding(f: Finding): string {
  const label = f.severity === "error" ? "エラー" : "警告";
  const lesson = f.lesson ? ` [${f.lesson}]` : "";
  return `${label} [${f.check}] ${f.file}${lesson}: ${f.message}`;
}

// 結果の表示と、終了コード（エラーが 1 件でもあれば 1、警告だけなら 0）。
export function report(findings: Finding[]): {
  text: string;
  exitCode: number;
} {
  const errors = findings.filter((f) => f.severity === "error");
  const warnings = findings.filter((f) => f.severity === "warning");
  const lines = [...errors, ...warnings].map(formatFinding);
  lines.push(`エラー ${errors.length} 件、警告 ${warnings.length} 件`);
  return { text: lines.join("\n"), exitCode: errors.length > 0 ? 1 : 0 };
}
