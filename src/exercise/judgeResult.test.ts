import { describe, expect, it } from "vitest";
import type { RunFinished, SubmitResult } from "../commands/run";
import {
  clockText,
  firstFailedCase,
  nextJudgeCard,
  passedAtText,
  visibleWhitespace,
  wrongSubText,
} from "./judgeResult";
import type { JudgeCardState, SubmitFinished } from "./judgeResult";

const AT = new Date(2026, 10, 24, 14, 32);

function result(
  status: SubmitResult["status"],
  passed = 0,
  total = 0,
): SubmitResult {
  return {
    status,
    passed,
    total,
    cases:
      status === "wrong"
        ? [
            { name: "基本", status: "pass", detail: null },
            { name: "空白", status: "fail", detail: "panic" },
          ]
        : [],
  };
}

function finished(
  submit: SubmitResult | null,
  error: RunFinished["error"] = null,
): SubmitFinished {
  return {
    event: {
      request_id: "r1",
      kind: "submit",
      finished_at: "2026-11-24T14:33:00+09:00",
      error,
      run: null,
      submit,
      progress: null,
    },
    submittedAt: AT,
    files: { "src/main.rs": "" },
  };
}

describe("判定結果のカード", () => {
  const wrongCard: JudgeCardState = {
    result: { ...result("wrong", 1, 2), status: "wrong" },
    submittedAt: AT,
    seq: 1,
  };

  it("不正解とビルドの失敗は、最新の結果に置き換え、番号を進める", () => {
    expect(nextJudgeCard(null, finished(result("wrong", 1, 2)))).toEqual({
      result: result("wrong", 1, 2),
      submittedAt: AT,
      seq: 1,
    });
    const next = nextJudgeCard(wrongCard, finished(result("compile_error")));
    expect(next?.result.status).toBe("compile_error");
    expect(next?.seq).toBe(2);
  });

  it("正解したら、カードを消す", () => {
    expect(nextJudgeCard(wrongCard, finished(result("passed", 2, 2)))).toBe(
      null,
    );
  });

  it.each(["crates_unavailable", "stopped"] as const)(
    "%s では、前のカードを残す",
    (status) => {
      expect(nextJudgeCard(wrongCard, finished(result(status)))).toBe(
        wrongCard,
      );
      expect(nextJudgeCard(null, finished(result(status)))).toBe(null);
    },
  );

  it("アプリの側の失敗では、前のカードを残す", () => {
    const error = { code: "internal" as const, message: "失敗", detail: null };
    expect(nextJudgeCard(wrongCard, finished(null, error))).toBe(wrongCard);
  });

  it("見出しの下の文に、通過数と提出した時刻を書く", () => {
    expect(wrongSubText(result("wrong", 1, 3), AT)).toBe(
      "3 件中 2 件のテストが失敗しました ・ 14:32 に提出",
    );
    expect(clockText(new Date(2026, 0, 1, 9, 5))).toBe("09:05");
  });

  it("最初に失敗したテストケースを選ぶ", () => {
    expect(firstFailedCase(result("wrong", 1, 2))?.name).toBe("空白");
    expect(firstFailedCase(result("passed", 2, 2))).toBe(null);
  });
});

describe("空白を見えるようにする", () => {
  it("行頭と行末の空白を ␣ にし、行の途中の空白は残す", () => {
    expect(visibleWhitespace("  ferris  ")).toEqual([
      { text: "␣␣", ws: true },
      { text: "ferris", ws: false },
      { text: "␣␣", ws: true },
    ]);
    expect(visibleWhitespace("a b\n c")).toEqual([
      { text: "a b\n", ws: false },
      { text: "␣", ws: true },
      { text: "c", ws: false },
    ]);
  });

  it("タブと CR も行末の空白として扱う", () => {
    expect(visibleWhitespace("x\t\r\ny")).toEqual([
      { text: "x", ws: false },
      { text: "␣␣", ws: true },
      { text: "\ny", ws: false },
    ]);
  });
});

describe("前回の正解コードの日時", () => {
  it("手元の時刻の月・日・時刻で書く", () => {
    const at = new Date(2026, 9, 28, 14, 32);
    expect(passedAtText(at.toISOString())).toBe("10 月 28 日 14:32 に正解");
    expect(passedAtText("壊れた日時")).toBe("正解");
  });
});
