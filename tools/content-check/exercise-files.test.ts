// 検査「問題文・解説」「テストケース」「クレート」（documents/spec/content-guidelines/check-tool.md）のテスト。
// エラーになる教材とならない教材を、一時ディレクトリに作って確かめる。
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  casesCheck,
  CASES_CHECK,
  cratesCheck,
  CRATES_CHECK,
  testFunctionNames,
  textCheck,
  TEXT_CHECK,
} from "./exercise-files.ts";
import type { Check, Finding } from "./types.ts";

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

const PROBLEM = "## 問題\n\n説明\n\n## 出力\n\n形式\n\n## 例\n\n例\n";
const PROBLEM_WITH_INPUT = PROBLEM.replace(
  "## 出力",
  "## 入力\n\n形式\n\n## 出力",
);
const EXPLANATION =
  "## 考え方\n\na\n\n## 模範解答\n\nb\n\n## ポイント\n\nc\n\n## よくある間違い\n\nd\n";

function cases(n: number, stdin = ""): string {
  return JSON.stringify(
    Array.from({ length: n }, (_, i) => ({
      name: `ケース${i}`,
      stdin,
      expected_stdout: "x\n",
    })),
  );
}

function tests(names: string[]): string {
  return [
    "use super::*;",
    "",
    ...names.flatMap((n) => ["#[test]", `fn ${n}() {}`, ""]),
  ].join("\n");
}

interface LessonSpec {
  chapter: string;
  kind?: string;
  mode?: "stdout" | "test";
  dependencies?: string[];
  caseLabels?: Record<string, string>;
  files?: Record<string, string>;
}

// 1 つのレッスンだけを持つ教材を作り、検査 `check` の結果を返す。
function run(
  check: Check,
  id: string,
  spec: LessonSpec,
  allowedCrates = '[dependencies]\nrand = "=0.9.1"\n',
): Finding[] {
  const dir = mkdtempSync(join(tmpdir(), "progrust-exercise-files-"));
  dirs.push(dir);
  const contentDir = join(dir, "content");
  const lessonDir = join(contentDir, "lessons", id);
  const write = (rel: string, text: string) => {
    mkdirSync(dirname(join(lessonDir, rel)), { recursive: true });
    writeFileSync(join(lessonDir, rel), text);
  };
  mkdirSync(contentDir, { recursive: true });
  writeFileSync(
    join(contentDir, "chapters.json"),
    JSON.stringify([{ id: spec.chapter, lessons: [id, "prepared"] }]),
  );
  const kind = spec.kind ?? "lesson";
  const lesson: Record<string, unknown> = { id, chapter: spec.chapter, kind };
  if (kind !== "video_only") {
    lesson.exercise = {
      layout: "single",
      judge: { mode: spec.mode ?? "stdout", target: "src/main.rs" },
      dependencies: spec.dependencies ?? [],
      readonly_files: [],
      case_labels: spec.caseLabels ?? {},
    };
  }
  write("lesson.json", JSON.stringify(lesson));
  const mode = spec.mode ?? "stdout";
  const defaults: Record<string, string> = {
    "exercise/problem.md": PROBLEM,
    "exercise/explanation.md": EXPLANATION,
    ...(mode === "stdout"
      ? { "exercise/cases.json": cases(3) }
      : { "exercise/judge_tests.rs": tests(["a", "b", "c"]) }),
  };
  for (const [rel, text] of Object.entries({ ...defaults, ...spec.files })) {
    write(rel, text);
  }
  const allowedCratesFile = join(dir, "allowed-crates.toml");
  writeFileSync(allowedCratesFile, allowedCrates);
  return check.run({
    contentDir,
    curriculumDir: join(dir, "chapters"),
    allowedCratesFile,
  });
}

describe("テストケース", () => {
  it("ch01 のテストケース 1 件は、エラーにならない", () => {
    expect(
      run(casesCheck, "ch01-01", {
        chapter: "ch01",
        files: { "exercise/cases.json": cases(1) },
      }),
    ).toEqual([]);
  });

  it("ch02 の章末復習レッスンのテストケース 1 件は、エラーにならない", () => {
    expect(
      run(casesCheck, "ch02-review", {
        chapter: "ch02",
        kind: "chapter_review",
        files: { "exercise/cases.json": cases(1) },
      }),
    ).toEqual([]);
  });

  it("ch03 のテストケース 2 件は、エラーになる", () => {
    const findings = run(casesCheck, "ch03-01", {
      chapter: "ch03",
      files: { "exercise/cases.json": cases(2) },
    });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: CASES_CHECK,
      file: "lessons/ch03-01/exercise/cases.json",
      lesson: "ch03-01",
    });
    expect(findings[0].message).toContain("2 件");
  });

  it("ch03 のテストケース 3 件は、エラーにならない", () => {
    expect(run(casesCheck, "ch03-01", { chapter: "ch03" })).toEqual([]);
  });

  it("ch03 の章末復習レッスンのテストケース 2 件は、エラーになる", () => {
    const findings = run(casesCheck, "ch03-review", {
      chapter: "ch03",
      kind: "chapter_review",
      files: { "exercise/cases.json": cases(2) },
    });
    expect(findings.map((f) => f.check)).toEqual([CASES_CHECK]);
  });

  it("test 方式は、#[test] の関数の数で数える", () => {
    const base = { chapter: "ch03", mode: "test" } as const;
    expect(run(casesCheck, "ch03-01", base)).toEqual([]);
    const findings = run(casesCheck, "ch03-01", {
      ...base,
      files: { "exercise/judge_tests.rs": tests(["a", "b"]) },
    });
    expect(findings).toHaveLength(1);
    expect(findings[0].file).toBe("lessons/ch03-01/exercise/judge_tests.rs");
  });

  it("case_labels に judge_tests.rs に無い関数名があると、エラーになる", () => {
    const findings = run(casesCheck, "ch03-01", {
      chapter: "ch03",
      mode: "test",
      caseLabels: { a: "表示名", missing_fn: "無い" },
    });
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("missing_fn");
  });

  it("case_labels のキーがすべてテスト関数名なら、エラーにならない", () => {
    expect(
      run(casesCheck, "ch03-01", {
        chapter: "ch03",
        mode: "test",
        caseLabels: { a: "A", b: "B" },
      }),
    ).toEqual([]);
  });

  it("動画のみのレッスンは、検査しない", () => {
    expect(
      run(casesCheck, "ch03-02", { chapter: "ch03", kind: "video_only" }),
    ).toEqual([]);
  });

  it("cases.json が配列でないと、エラーになる", () => {
    const findings = run(casesCheck, "ch03-01", {
      chapter: "ch03",
      files: { "exercise/cases.json": "{}" },
    });
    expect(findings).toHaveLength(1);
  });

  it("テスト関数名を、属性やコメントをはさんでも読める", () => {
    expect(
      testFunctionNames(
        "#[test]\n#[should_panic]\n// c\nfn a() {}\n#[test]\npub fn b() {}\nfn helper() {}\n",
      ),
    ).toEqual(["a", "b"]);
  });
});

describe("問題文・解説", () => {
  it("必須の見出しがそろっていれば、エラーにならない", () => {
    expect(run(textCheck, "ch03-01", { chapter: "ch03" })).toEqual([]);
  });

  it("problem.md に必須の見出しが無いと、エラーになる", () => {
    const findings = run(textCheck, "ch03-01", {
      chapter: "ch03",
      files: { "exercise/problem.md": "## 問題\n\n説明\n" },
    });
    expect(findings.map((f) => f.message)).toEqual([
      "problem.md に見出し「## 出力」がない",
      "problem.md に見出し「## 例」がない",
    ]);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: TEXT_CHECK,
      file: "lessons/ch03-01/exercise/problem.md",
    });
  });

  it("explanation.md に必須の見出しが無いと、エラーになる", () => {
    const findings = run(textCheck, "ch03-01", {
      chapter: "ch03",
      files: {
        "exercise/explanation.md": EXPLANATION.replace(
          "## ポイント",
          "### ポイント",
        ),
      },
    });
    expect(findings.map((f) => f.message)).toEqual([
      "explanation.md に見出し「## ポイント」がない",
    ]);
  });

  it("コードブロックの中の見出しは、見出しと数えない", () => {
    const findings = run(textCheck, "ch03-01", {
      chapter: "ch03",
      files: {
        "exercise/problem.md": "## 問題\n\n```\n## 出力\n## 例\n```\n",
      },
    });
    expect(findings).toHaveLength(2);
  });

  it("入力のある stdout 方式では、「## 入力」が必須になる", () => {
    const files = { "exercise/cases.json": cases(3, "1 2\n") };
    const findings = run(textCheck, "ch03-01", {
      chapter: "ch03",
      files,
    });
    expect(findings.map((f) => f.message)).toEqual([
      "problem.md に見出し「## 入力」がない",
    ]);
    expect(
      run(textCheck, "ch03-01", {
        chapter: "ch03",
        files: { ...files, "exercise/problem.md": PROBLEM_WITH_INPUT },
      }),
    ).toEqual([]);
  });

  it("入力のない演習と test 方式では、「## 入力」も「## 制約」も必須でない", () => {
    expect(
      run(textCheck, "ch03-01", { chapter: "ch03", mode: "test" }),
    ).toEqual([]);
  });

  it("動画のみのレッスンは、検査しない", () => {
    expect(
      run(textCheck, "ch03-02", { chapter: "ch03", kind: "video_only" }),
    ).toEqual([]);
  });
});

describe("クレート", () => {
  it("許可クレートの一覧にあるクレートは、エラーにならない", () => {
    expect(
      run(cratesCheck, "ch03-01", { chapter: "ch03", dependencies: ["rand"] }),
    ).toEqual([]);
  });

  it("dependencies が空なら、エラーにならない", () => {
    expect(run(cratesCheck, "ch03-01", { chapter: "ch03" })).toEqual([]);
  });

  it("許可クレートの一覧に無いクレートは、エラーになる", () => {
    const findings = run(cratesCheck, "ch03-01", {
      chapter: "ch03",
      dependencies: ["rand", "serde"],
    });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: CRATES_CHECK,
      file: "lessons/ch03-01/lesson.json",
    });
    expect(findings[0].message).toContain("serde");
  });
});
