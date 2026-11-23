// `npm run content:check` の入口。
// 引数: --content <教材のディレクトリ> --curriculum <章とレッスンの一覧のディレクトリ>
import { fileURLToPath } from "node:url";
import { report, runChecks } from "./run.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));

function option(name: string, fallback: string): string {
  const i = process.argv.indexOf(name);
  return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : fallback;
}

const findings = runChecks({
  contentDir: option("--content", `${root}content`),
  curriculumDir: option(
    "--curriculum",
    `${root}documents/spec/curriculum/chapters`,
  ),
});
const { text, exitCode } = report(findings);
console.log(text);
process.exitCode = exitCode;
