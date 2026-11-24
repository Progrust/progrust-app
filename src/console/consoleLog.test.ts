import { describe, expect, it } from "vitest";
import type { RunOutputChunk } from "../commands/run";
import {
  CONSOLE_TAIL_LIMIT,
  EMPTY_CONSOLE_LOG,
  appendInput,
  appendRunOutput,
  type ConsoleLog,
} from "./consoleLog";

const output = (chunks: RunOutputChunk[], requestId = "r1") => ({
  request_id: requestId,
  chunks,
});

const text = (log: ConsoleLog) => log.segments.map((s) => s.text).join("");

// 種類と色を、見比べやすい形にする。
const show = (log: ConsoleLog) =>
  log.segments.map(
    (s) =>
      `${s.kind}/${s.style.bold ? "b" : ""}${s.style.color ?? "-"}:${s.text}`,
  );

describe("足していく", () => {
  it("出力の断片と入力した行を、受け取った順に足す", () => {
    let log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([
        { stream: "stderr", text: "\u001b[1m\u001b[93mwarning\u001b[0m: x\n" },
        { stream: "stdout", text: "名前: " },
      ]),
    );
    log = appendInput(log, "ferris\n");
    log = appendRunOutput(
      log,
      output([
        { stream: "stdout", text: "こんにちは ferris\n" },
        { stream: "system", text: "終了しました" },
      ]),
    );
    expect(show(log)).toEqual([
      "stderr/byellow:warning",
      "stderr/-:: x\n",
      "stdout/-:名前: ",
      "input/-:ferris\n",
      "stdout/-:こんにちは ferris\n",
      "system/-:終了しました",
    ]);
    expect(log.length).toBe(text(log).length);
    expect(log.truncated).toBe(false);
  });

  it("種類と色が同じものはつなげ、id を変えない", () => {
    const first = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([{ stream: "stdout", text: "a\n" }]),
    );
    const second = appendRunOutput(
      first,
      output([
        { stream: "stdout", text: "b\n" },
        { stream: "stdout", text: "c\n" },
      ]),
    );
    expect(second.segments).toHaveLength(1);
    expect(second.segments[0].id).toBe(first.segments[0].id);
    expect(second.segments[0].text).toBe("a\nb\nc\n");
    // 元の log は変えない。
    expect(first.segments[0].text).toBe("a\n");
  });

  it("入力した行のエスケープは読まない", () => {
    const log = appendInput(EMPTY_CONSOLE_LOG, "[31mx\n");
    expect(show(log)).toEqual(["input/-:[31mx\n"]);
  });

  it("色とエスケープの読みかけを、種類ごとに持ち越す", () => {
    let log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([
        { stream: "stderr", text: "\u001b[1m\u001b[9" },
        { stream: "stdout", text: "\u001b[32mok" },
      ]),
    );
    log = appendRunOutput(
      log,
      output([
        { stream: "stderr", text: "1merror\u001b[0m" },
        { stream: "stdout", text: " done" },
      ]),
    );
    expect(show(log)).toEqual([
      "stdout/green:ok",
      "stderr/bred:error",
      "stdout/green: done",
    ]);
  });

  it("別の実行・提出の出力では、前の色と読みかけを持ち越さない", () => {
    let log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([{ stream: "stdout", text: "\u001b[31mred\u001b[" }], "r1"),
    );
    log = appendRunOutput(
      log,
      output([{ stream: "stdout", text: "plain" }], "r2"),
    );
    expect(show(log)).toEqual(["stdout/red:red", "stdout/-:plain"]);
    expect(log.requestId).toBe("r2");
  });
});

describe("末尾の 64K 文字だけを残す", () => {
  it("上限ちょうどまでは消さない", () => {
    const log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([{ stream: "stdout", text: "a".repeat(CONSOLE_TAIL_LIMIT) }]),
    );
    expect(log.length).toBe(CONSOLE_TAIL_LIMIT);
    expect(log.truncated).toBe(false);
  });

  it("超えたら古いほうから消し、消したことを覚えておく", () => {
    let log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([{ stream: "stdout", text: "a".repeat(CONSOLE_TAIL_LIMIT) }]),
    );
    log = appendRunOutput(log, output([{ stream: "stderr", text: "bc" }]));
    expect(log.truncated).toBe(true);
    expect(log.length).toBe(CONSOLE_TAIL_LIMIT);
    expect(text(log)).toBe("a".repeat(CONSOLE_TAIL_LIMIT - 2) + "bc");

    // 少ない出力を足しても、消したことは忘れない。
    log = appendInput(log, "x\n");
    expect(log.truncated).toBe(true);
  });

  it("消す長さは、エスケープを除いた表示する文字で数え、色を保つ", () => {
    const line = "\u001b[1m\u001b[91merror\u001b[0m: x\n"; // 表示するのは 9 文字
    const count = Math.ceil(CONSOLE_TAIL_LIMIT / 9) + 100;
    const log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([{ stream: "stderr", text: line.repeat(count) }]),
    );
    expect(log.length).toBe(CONSOLE_TAIL_LIMIT);
    expect(text(log)).toBe(
      "error: x\n".repeat(count).slice(-CONSOLE_TAIL_LIMIT),
    );
    const last = log.segments.slice(-2);
    expect(show({ ...log, segments: last })).toEqual([
      "stderr/bred:error",
      "stderr/-:: x\n",
    ]);
  });

  it("サロゲートペアの途中で切らない", () => {
    const log = appendRunOutput(
      EMPTY_CONSOLE_LOG,
      output([
        { stream: "stdout", text: "a🦀" + "b".repeat(CONSOLE_TAIL_LIMIT - 1) },
      ]),
    );
    // 「a」と、🦀 の前半だけを消すと壊れるので、🦀 ごと消す。
    expect(text(log)).toBe("b".repeat(CONSOLE_TAIL_LIMIT - 1));
    expect(log.length).toBe(CONSOLE_TAIL_LIMIT - 1);
  });

  // 1 MB（実行 1 回の出力量の上限）を、技術検証の 4 種類の出力で送る（ADR 0103）。
  const MB = 1024 * 1024;
  const cases: [string, () => RunOutputChunk[][]][] = [
    [
      "短い行（a と改行）を 1 つの断片で",
      () => [[{ stream: "stdout", text: "a\n".repeat(MB / 2) }]],
    ],
    [
      "40 バイトの行を 16 回に分けて",
      () =>
        Array.from({ length: 16 }, (_, n) => [
          {
            stream: "stdout" as const,
            text: Array.from(
              { length: MB / 16 / 40 },
              (_, i) =>
                `line ${String(n * 10000 + i).padStart(8, "0")} ${"x".repeat(25)}\n`,
            ).join(""),
          },
        ]),
    ],
    ["改行のない 1 MB", () => [[{ stream: "stdout", text: "z".repeat(MB) }]]],
    [
      "stdout と stderr を交互に",
      () =>
        Array.from({ length: 8 }, (_, n) =>
          Array.from({ length: 4096 }, (_, i) => ({
            stream: i % 2 === 0 ? ("stdout" as const) : ("stderr" as const),
            text: `${n}-${i}`.padEnd(31, ".") + "\n",
          })),
        ),
    ],
  ];

  it.each(cases)("%s: 末尾だけが残る", (_, make) => {
    const events = make();
    const all = events
      .flat()
      .map((c) => c.text)
      .join("");
    let log = EMPTY_CONSOLE_LOG;
    for (const chunks of events) {
      log = appendRunOutput(log, output(chunks));
    }
    expect(log.truncated).toBe(true);
    expect(log.length).toBe(CONSOLE_TAIL_LIMIT);
    expect(text(log)).toBe(all.slice(-CONSOLE_TAIL_LIMIT));
    // 描く span の数も、残した文字数で抑えられる。
    expect(log.segments.length).toBeLessThanOrEqual(
      CONSOLE_TAIL_LIMIT / 32 + 1,
    );
  });
});
