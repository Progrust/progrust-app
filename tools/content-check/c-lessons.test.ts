// C のレッスンの検査（documents/spec/content-guidelines/check-tool.md）のテスト。
// C のレッスンの見本（テスト用の教材）を一時ディレクトリに作り、エラーになる場合とならない場合を確かめる。
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { runChecks } from "./run.ts";
import type { Finding } from "./types.ts";

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

type Json = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any

const PROBLEM = "## 問題\n\n## 出力\n\n## 例\n";
const EXPLANATION =
  "## 考え方\n\n## 模範解答\n\n## ポイント\n\n## よくある間違い\n";

function cases(n: number): string {
  return JSON.stringify(
    Array.from({ length: n }, (_, i) => ({
      name: `ケース${i}`,
      stdin: "",
      expected_stdout: "x\n",
    })),
  );
}

function cLesson(id: string): Json {
  return {
    id,
    chapter: id.slice(0, 3),
    kind: "lesson",
    title: `タイトル${id}`,
    summary: "説明",
    tags: ["タグ"],
    primary_elements: [],
    secondary_elements: [],
    estimated_minutes: 10,
    exercise: {
      layout: "single",
      judge: { mode: "stdout", target: "main.c" },
      dependencies: [],
      readonly_files: [],
      case_labels: {},
    },
  };
}

interface Options {
  change?: (lesson: Json) => void;
  files?: (files: Record<string, string | undefined>) => void;
}

// C の章 c01 に、レッスン `id` 1 つだけを置いた教材を作り、すべての検査の結果を返す。
// 動画の検査の結果は、ここでは見ない（動画のファイルを置かないので「構成」のエラーが出る）。
function run(id: string, options: Options = {}): Finding[] {
  const dir = mkdtempSync(join(tmpdir(), "progrust-c-lessons-"));
  dirs.push(dir);
  const contentDir = join(dir, "content");
  const curriculumDir = join(dir, "chapters");
  mkdirSync(contentDir, { recursive: true });
  mkdirSync(curriculumDir);
  writeFileSync(
    join(contentDir, "chapters.json"),
    JSON.stringify([
      { id: id.slice(0, 3), language: "c", lessons: [id, "c01-prepared"] },
    ]),
  );
  writeFileSync(join(contentDir, "elements.json"), "[]");
  const lesson = cLesson(id);
  options.change?.(lesson);
  const files: Record<string, string | undefined> = {
    "lesson.json": JSON.stringify(lesson),
    "exercise/problem.md": PROBLEM,
    "exercise/explanation.md": EXPLANATION,
    "exercise/starter/main.c": "",
    "exercise/solution/main.c": "",
    "exercise/cases.json": cases(3),
  };
  options.files?.(files);
  for (const [rel, text] of Object.entries(files)) {
    if (text === undefined) continue;
    const path = join(contentDir, "lessons", id, rel);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, text);
  }
  return runChecks({ contentDir, curriculumDir }).filter(
    (f) =>
      f.lesson === id &&
      ["構成", "定義", "テストケース", "クレート", "問題文・解説"].includes(
        f.check,
      ) &&
      // 動画のファイルは、ここでは置かない。
      !f.file.includes("/video/"),
  );
}

function messages(findings: Finding[], check: string): string[] {
  return findings.filter((f) => f.check === check).map((f) => f.message);
}

describe("C のレッスンの lesson.json（定義）", () => {
  it("要素が空の C のレッスンは、エラーにならない", () => {
    expect(run("c01-05")).toEqual([]);
  });

  it.each(["primary_elements", "secondary_elements"])(
    "%s が空でないと、エラーになる",
    (key) => {
      const findings = run("c01-05", {
        change: (l) => (l[key] = ["x.y"]),
      });
      expect(messages(findings, "定義")).toEqual([
        expect.stringContaining(`${key} は空の配列`),
      ]);
    },
  );

  it("C のレッスンの要素は、要素マスタに無くても、空であればエラーにならない", () => {
    expect(messages(run("c02-01"), "定義")).toEqual([]);
  });

  it("kind が generated だと、エラーになる", () => {
    const findings = run("c01-05", { change: (l) => (l.kind = "generated") });
    expect(messages(findings, "定義")).toContain(
      "C のレッスンの kind に generated は使えない",
    );
  });

  it("章末復習レッスンは、主要素が空でエラーにならず、tags を持つとエラーになる", () => {
    const review = (l: Json) => {
      l.kind = "chapter_review";
      delete l.tags;
    };
    expect(messages(run("c01-review", { change: review }), "定義")).toEqual([]);
    const findings = run("c01-review", {
      change: (l) => {
        review(l);
        l.tags = ["x"];
      },
    });
    expect(messages(findings, "定義")).toEqual([
      expect.stringContaining("tags を持たない"),
    ]);
  });

  it.each<[string, (exercise: Json) => void]>([
    ["judge.mode が test", (e) => (e.judge.mode = "test")],
    ["layout が multi", (e) => (e.layout = "multi")],
    ["judge.target が src/main.rs", (e) => (e.judge.target = "src/main.rs")],
    ["judge.target が lib.c", (e) => (e.judge.target = "lib.c")],
    ["min_learner_tests がある", (e) => (e.judge.min_learner_tests = 1)],
    ["dependencies が空でない", (e) => (e.dependencies = ["rand"])],
    ["readonly_files が空でない", (e) => (e.readonly_files = ["a.c"])],
    ["case_labels が空でない", (e) => (e.case_labels = { a: "A" })],
  ])("exercise で %s と、エラーになる", (_name, change) => {
    const findings = run("c01-05", { change: (l) => change(l.exercise) });
    expect(messages(findings, "定義").length).toBeGreaterThanOrEqual(1);
  });

  it("exercise の layout・judge の書き方が Rust の形（src/main.rs）だと、エラーになる", () => {
    const findings = run("c01-05", {
      change: (l) => {
        l.exercise.judge = { mode: "stdout", target: "src/main.rs" };
      },
    });
    expect(messages(findings, "定義")).toEqual([
      expect.stringContaining("main.c"),
    ]);
  });
});

describe("C の演習のファイル（構成）", () => {
  it("main.c と cases.json だけの演習は、エラーにならない", () => {
    expect(messages(run("c01-05"), "構成")).toEqual([]);
  });

  it.each([
    "exercise/starter/main.c",
    "exercise/solution/main.c",
    "exercise/cases.json",
  ])("%s が無いと、エラーになる", (path) => {
    const findings = run("c01-05", {
      files: (files) => (files[path] = undefined),
    });
    expect(findings.map((f) => f.file)).toEqual([`lessons/c01-05/${path}`]);
  });

  it("Rust の starter/src/main.rs（ほか src/・Cargo.toml）があると、エラーになる", () => {
    const findings = run("c01-05", {
      files: (files) => {
        files["exercise/starter/src/main.rs"] = "";
        files["exercise/solution/Cargo.toml"] = "";
      },
    });
    expect(findings.map((f) => f.file)).toEqual([
      "lessons/c01-05/exercise/starter/src",
      "lessons/c01-05/exercise/solution/Cargo.toml",
    ]);
    expect(findings.every((f) => f.check === "構成")).toBe(true);
  });

  it("judge_tests.rs があると、エラーになる", () => {
    const findings = run("c01-05", {
      files: (files) => (files["exercise/judge_tests.rs"] = ""),
    });
    expect(findings).toEqual([
      expect.objectContaining({
        check: "構成",
        file: "lessons/c01-05/exercise/judge_tests.rs",
      }),
    ]);
  });
});

describe("C の演習のテストケース", () => {
  const withCases = (n: number) => ({
    files: (files: Record<string, string | undefined>) =>
      (files["exercise/cases.json"] = cases(n)),
  });

  it("3 件はエラーにならず、2 件はエラーになる", () => {
    expect(messages(run("c01-05", withCases(3)), "テストケース")).toEqual([]);
    expect(messages(run("c01-05", withCases(2)), "テストケース")).toEqual([
      expect.stringContaining("2 件"),
    ]);
    expect(messages(run("c03-01", withCases(2)), "テストケース")).toHaveLength(
      1,
    );
  });

  it.each(["c01-02", "c01-03", "c01-04"])(
    "%s は 1 件でエラーにならない",
    (id) => {
      expect(messages(run(id, withCases(1)), "テストケース")).toEqual([]);
    },
  );

  it("c01-02 でも 0 件はエラーになる", () => {
    expect(messages(run("c01-02", withCases(0)), "テストケース")).toHaveLength(
      1,
    );
  });

  it("c01-05 は 1 件でエラーになる", () => {
    expect(messages(run("c01-05", withCases(1)), "テストケース")).toHaveLength(
      1,
    );
  });
});

describe("Rust のレッスンだけの検査", () => {
  it("C のレッスンは、dependencies の検査（クレート）の対象にならない", () => {
    const findings = run("c01-05", {
      change: (l) => (l.exercise.dependencies = ["not-allowed"]),
    });
    expect(messages(findings, "クレート")).toEqual([]);
  });
});

describe("要素の検査は Rust のレッスンだけ", () => {
  // C の章に準備中のレッスンがあり、Rust の章のレッスンはすべて揃っている教材。
  function runMaster(cSecondary: string[]): Finding[] {
    const dir = mkdtempSync(join(tmpdir(), "progrust-c-lessons-"));
    dirs.push(dir);
    const contentDir = join(dir, "content");
    mkdirSync(join(contentDir, "lessons/c01-01"), { recursive: true });
    mkdirSync(join(contentDir, "lessons/ch01-01"), { recursive: true });
    writeFileSync(
      join(contentDir, "chapters.json"),
      JSON.stringify([
        { id: "c01", language: "c", lessons: ["c01-01", "c01-prepared"] },
        { id: "ch01", lessons: ["ch01-01"] },
      ]),
    );
    writeFileSync(
      join(contentDir, "elements.json"),
      JSON.stringify([
        { id: "x.a", chapter: "ch01" },
        { id: "x.unused", chapter: "ch01" },
      ]),
    );
    writeFileSync(
      join(contentDir, "lessons/c01-01/lesson.json"),
      JSON.stringify({ ...cLesson("c01-01"), secondary_elements: cSecondary }),
    );
    writeFileSync(
      join(contentDir, "lessons/ch01-01/lesson.json"),
      JSON.stringify({
        id: "ch01-01",
        chapter: "ch01",
        kind: "lesson",
        primary_elements: ["x.a"],
        secondary_elements: [],
      }),
    );
    return runChecks({ contentDir, curriculumDir: join(dir, "chapters") });
  }

  it("C の章に準備中のレッスンがあっても、要素マスタの検査は行う", () => {
    expect(messages(runMaster([]), "要素マスタ")).toEqual([
      expect.stringContaining("x.unused"),
    ]);
  });

  it("C のレッスンの secondary_elements は、既習要素の検査の対象にならない", () => {
    expect(messages(runMaster(["x.a"]), "既習要素")).toEqual([]);
  });
});
