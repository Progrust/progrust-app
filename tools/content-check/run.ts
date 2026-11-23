// 教材チェックツールの本体。検査の項目は checks に足していく。
import { audioCheck } from "./audio.ts";
import { curriculumCheck } from "./curriculum.ts";
import { definitionCheck } from "./definition.ts";
import { elementsCheck } from "./elements.ts";
import {
  learnedElementsCheck,
  masterElementsUsedCheck,
  reviewPrimaryElementsCheck,
} from "./elements-in-lessons.ts";
import { lintCheck } from "./lint.ts";
import { checkRender } from "./render.ts";
import { scriptCheck } from "./script.ts";
import { structureCheck } from "./structure.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";
import { warningsCheck } from "./warnings.ts";

export const checks: Check[] = [
  curriculumCheck,
  structureCheck,
  definitionCheck,
  learnedElementsCheck,
  masterElementsUsedCheck,
  elementsCheck,
  reviewPrimaryElementsCheck,
  scriptCheck,
  audioCheck,
  lintCheck,
  warningsCheck,
];

export function runChecks(
  options: CheckOptions,
  list: Check[] = checks,
): Finding[] {
  return list.flatMap((check) => check.run(options));
}

// checks のすべての項目と、動画を描画する「描画」「表紙」（render.ts）を検査する。
// 描画は時間がかかり、非同期なので、checks に並べずに、ここで続けて行う。
export async function runAllChecks(options: CheckOptions): Promise<Finding[]> {
  return [...runChecks(options), ...(await checkRender(options))];
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
