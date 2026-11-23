// 教材チェックツールの検査「模範解答」「初期コード」（documents/spec/content-guidelines/check-tool.md）のテスト。
// エラーになる教材とならない教材を一時ディレクトリに作り、実物の cargo で判定する。
// 判定だけを行うプログラムを cargo run でビルドするので、はじめての実行には数分かかることがある。
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { checkJudge, SOLUTION_CHECK, STARTER_CHECK } from "./judge.ts";
import type { Finding } from "./types.ts";

// CI でキャッシュが無いときは、Tauri を含むクレート全体を debug でビルドするので、長めにする。
const JUDGE_TIMEOUT = 1_200_000;

type Files = Record<string, string>;

interface LessonSpec {
  kind?: string;
  exercise?: Record<string, unknown>;
  // レッスンのディレクトリからの相対パスと、内容。
  files: Files;
}

const STDOUT_EXERCISE = {
  layout: "single",
  judge: { mode: "stdout", target: "src/main.rs" },
  dependencies: [],
  readonly_files: [],
  case_labels: {},
};

const CASES = JSON.stringify([
  { name: "基本", stdin: "2 3\n", expected_stdout: "5\n" },
  { name: "負の数", stdin: "-1 1\n", expected_stdout: "0\n" },
]);

// 標準入力の 2 つの整数の和を出力するプログラム。`expr` で和の式を差し替える。
function addProgram(expr: string): string {
  return [
    "use std::io::Read;",
    "",
    "fn main() {",
    "    let mut input = String::new();",
    "    std::io::stdin().read_to_string(&mut input).unwrap();",
    "    let n: Vec<i64> = input.split_whitespace().map(|s| s.parse().unwrap()).collect();",
    "    let (a, b) = (n[0], n[1]);",
    `    println!("{}", ${expr});`,
    "}",
    "",
  ].join("\n");
}

function stdoutLesson(solution: string, starter: string): LessonSpec {
  return {
    exercise: STDOUT_EXERCISE,
    files: {
      "exercise/cases.json": CASES,
      "exercise/solution/src/main.rs": solution,
      "exercise/starter/src/main.rs": starter,
    },
  };
}

const TEST_SOLUTION = [
  "fn add(a: i32, b: i32) -> i32 {",
  "    a + b",
  "}",
  "",
  "fn main() {",
  '    println!("{}", add(1, 2));',
  "}",
  "",
  "#[cfg(test)]",
  "mod tests {",
  "    use super::*;",
  "",
  "    #[test]",
  "    fn my_add() {",
  "        assert_eq!(add(2, 2), 4);",
  "    }",
  "}",
  "",
].join("\n");

const TEST_STARTER = [
  "fn add(_a: i32, _b: i32) -> i32 {",
  "    todo!()",
  "}",
  "",
  "fn main() {",
  '    println!("{}", add(1, 2));',
  "}",
  "",
].join("\n");

const lessons: Record<string, LessonSpec> = {
  // 模範解答が通り、初期コードが正解にならない（エラーにならない）。
  "ok-stdout": stdoutLesson(addProgram("a + b"), addProgram("a - b")),
  // test 方式と、学習者のテストの判定（エラーにならない）。
  "ok-test": {
    exercise: {
      layout: "single",
      judge: { mode: "test", target: "src/main.rs", min_learner_tests: 1 },
      dependencies: [],
      readonly_files: [],
      case_labels: { adds: "足し算" },
    },
    files: {
      "exercise/judge_tests.rs": [
        "use super::*;",
        "",
        "#[test]",
        "fn adds() {",
        "    assert_eq!(add(1, 2), 3);",
        "}",
        "",
      ].join("\n"),
      "exercise/solution/src/main.rs": TEST_SOLUTION,
      "exercise/starter/src/main.rs": TEST_STARTER,
    },
  },
  // 模範解答が 1 件のテストケースに通らない。
  "wrong-solution": stdoutLesson(
    addProgram("if a < 0 { a * b } else { a + b }"),
    addProgram("a - b"),
  ),
  // 模範解答がコンパイルできない。
  "broken-solution": stdoutLesson(
    addProgram("a + b + undefined_value"),
    addProgram("a - b"),
  ),
  // 初期コードが正解になる。
  "passing-starter": stdoutLesson(addProgram("a + b"), addProgram("b + a")),
  // 許可クレートを使う演習は、まだ判定しない（模範解答が通らなくてもエラーにならない）。
  "uses-crates": {
    exercise: { ...STDOUT_EXERCISE, dependencies: ["rand"] },
    files: {
      "exercise/cases.json": CASES,
      "exercise/solution/src/main.rs": addProgram("a - b"),
      "exercise/starter/src/main.rs": addProgram("a + b"),
    },
  },
  // 動画のみのレッスンは判定しない。
  "video-only": { kind: "video_only", files: {} },
};

// chapters.json に載せるが、ディレクトリを作らない（準備中のレッスン）。
const PREPARING = "preparing";

let contentDir = "";
let workspaceDir = "";
let findings: Finding[] = [];

function write(path: string, content: string) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}

beforeAll(() => {
  contentDir = mkdtempSync(join(tmpdir(), "progrust-judge-check-"));
  workspaceDir = mkdtempSync(join(tmpdir(), "progrust-judge-workspace-"));
  write(
    join(contentDir, "chapters.json"),
    JSON.stringify([
      {
        id: "ch01",
        title: "章",
        description: "説明",
        lessons: [...Object.keys(lessons), PREPARING],
      },
    ]),
  );
  for (const [id, spec] of Object.entries(lessons)) {
    const dir = join(contentDir, "lessons", id);
    const lesson: Record<string, unknown> = {
      id,
      chapter: "ch01",
      kind: spec.kind ?? "lesson",
      title: id,
    };
    if (spec.exercise) lesson.exercise = spec.exercise;
    write(join(dir, "lesson.json"), JSON.stringify(lesson));
    for (const [path, content] of Object.entries(spec.files)) {
      write(join(dir, path), content);
    }
  }
  findings = checkJudge({
    contentDir,
    curriculumDir: contentDir,
    judgeWorkspaceDir: workspaceDir,
  });
}, JUDGE_TIMEOUT);

afterAll(() => {
  rmSync(contentDir, { recursive: true, force: true });
  rmSync(workspaceDir, { recursive: true, force: true });
});

function ofLesson(lesson: string): Finding[] {
  return findings.filter((f) => f.lesson === lesson);
}

describe("模範解答", () => {
  it("模範解答がすべてのテストケースに通るレッスンは、エラーにならない", () => {
    expect(ofLesson("ok-stdout")).toEqual([]);
    expect(ofLesson("ok-test")).toEqual([]);
  });

  it("模範解答がテストケースに通らないと、失敗したテストケースを示してエラーになる", () => {
    expect(ofLesson("wrong-solution")).toEqual([
      {
        severity: "error",
        check: SOLUTION_CHECK,
        file: "lessons/wrong-solution/exercise/solution",
        lesson: "wrong-solution",
        message: expect.stringContaining(
          "wrong、1/2 件通過。失敗したテストケース: 負の数（fail）",
        ),
      },
    ]);
  });

  it("模範解答がコンパイルできないと、ビルドの出力を示してエラーになる", () => {
    const errors = ofLesson("broken-solution");
    expect(errors).toEqual([
      expect.objectContaining({
        check: SOLUTION_CHECK,
        file: "lessons/broken-solution/exercise/solution",
      }),
    ]);
    expect(errors[0].message).toContain("compile_error、0/0 件通過");
    expect(errors[0].message).toContain("undefined_value");
  });
});

describe("初期コード", () => {
  it("初期コードが判定で正解になると、エラーになる", () => {
    expect(ofLesson("passing-starter")).toEqual([
      {
        severity: "error",
        check: STARTER_CHECK,
        file: "lessons/passing-starter/exercise/starter",
        lesson: "passing-starter",
        message: expect.stringContaining("passed、2/2 件通過"),
      },
    ]);
  });
});

describe("判定しないレッスン", () => {
  it("許可クレートを使う演習、動画のみのレッスン、準備中のレッスンは判定しない", () => {
    expect(ofLesson("uses-crates")).toEqual([]);
    expect(ofLesson("video-only")).toEqual([]);
    expect(ofLesson(PREPARING)).toEqual([]);
  });

  it("ほかのレッスンのエラーは無い", () => {
    const lessonsWithErrors = new Set(findings.map((f) => f.lesson));
    expect([...lessonsWithErrors].sort()).toEqual([
      "broken-solution",
      "passing-starter",
      "wrong-solution",
    ]);
  });
});

describe("判定できないとき", () => {
  it(
    "判定だけを行うプログラムが失敗すると、その検査のエラーになる",
    () => {
      const dir = mkdtempSync(join(tmpdir(), "progrust-judge-check-broken-"));
      try {
        write(
          join(dir, "chapters.json"),
          JSON.stringify([
            { id: "ch01", title: "章", description: "説明", lessons: ["bad"] },
          ]),
        );
        // stdout 方式なのに cases.json が無い。
        write(
          join(dir, "lessons/bad/lesson.json"),
          JSON.stringify({
            id: "bad",
            chapter: "ch01",
            kind: "lesson",
            title: "bad",
            exercise: STDOUT_EXERCISE,
          }),
        );
        write(join(dir, "lessons/bad/exercise/solution/src/main.rs"), "");
        write(join(dir, "lessons/bad/exercise/starter/src/main.rs"), "");
        const result = checkJudge({
          contentDir: dir,
          curriculumDir: dir,
          judgeWorkspaceDir: workspaceDir,
        });
        expect(result).toEqual(
          [SOLUTION_CHECK, STARTER_CHECK].map((check) =>
            expect.objectContaining({
              severity: "error",
              check,
              lesson: "bad",
              message: expect.stringContaining("判定できなかった"),
            }),
          ),
        );
        expect(result[0].message).toContain("cases.json");
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    },
    JUDGE_TIMEOUT,
  );
});
