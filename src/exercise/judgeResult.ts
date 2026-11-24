// 判定結果のカードに出すもの（documents/spec/screens/exercise-screen.md「判定結果」、
// documents/spec/exercise-runner/submit.md「判定結果」「判定をしなかった提出」「途中で止めた提出」、ADR 0015、ADR 0045）。
// 画面の組み立てから分けて、自動テストで確かめられるようにする。
import type {
  CaseResult,
  RunFinished,
  StdoutFailDetail,
  SubmitResult,
} from "../commands/run";
import type { DraftFiles } from "../commands/progress";

// 提出が終わったときに、コンソールの部品から受け取るもの。
export type SubmitFinished = {
  event: RunFinished;
  // 「提出」を押した時刻。
  submittedAt: Date;
  // submit_code に渡したファイル。
  files: DraftFiles;
};

// いま表示しているカード。不正解（wrong）とビルドの失敗（compile_error）だけを表示する。
export type JudgeCardState = {
  result: SubmitResult & { status: "wrong" | "compile_error" };
  submittedAt: Date;
  // 提出ごとに増やす番号。カードを作り直して、詳細を畳んだ状態に戻すために使う。
  seq: number;
};

// 提出が終わった後のカード。
// - wrong・compile_error: 最新の結果に置き換える（詳細は畳む）。
// - passed: カードを消す（正解は、正解のモーダルで示す）。
// - crates_unavailable・stopped・アプリの側の失敗: 判定をしていないので、前のカードをそのまま残す。
export function nextJudgeCard(
  current: JudgeCardState | null,
  finished: SubmitFinished,
): JudgeCardState | null {
  const { event, submittedAt } = finished;
  const result = event.submit;
  if (event.error !== null || result === null) {
    return current;
  }
  switch (result.status) {
    case "wrong":
    case "compile_error":
      return {
        result: { ...result, status: result.status },
        submittedAt,
        seq: (current?.seq ?? 0) + 1,
      };
    case "passed":
      return null;
    case "crates_unavailable":
    case "stopped":
      return current;
  }
}

// 時刻の時と分。例: 「14:32」。
export function clockText(at: Date): string {
  const hh = String(at.getHours()).padStart(2, "0");
  const mm = String(at.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}

// 前回の正解コードのモーダルに出す、正解した日時の文。例: 「10 月 28 日 14:32 に正解」。日時は手元の時刻で表す。
export function passedAtText(submittedAt: string): string {
  const at = new Date(submittedAt);
  if (Number.isNaN(at.getTime())) {
    return "正解";
  }
  return `${at.getMonth() + 1} 月 ${at.getDate()} 日 ${clockText(at)} に正解`;
}

// 不正解のカードの見出しの下の文。例: 「3 件中 1 件のテストが失敗しました ・ 14:32 に提出」。
export function wrongSubText(result: SubmitResult, submittedAt: Date): string {
  const failed = result.total - result.passed;
  return `${result.total} 件中 ${failed} 件のテストが失敗しました ・ ${clockText(submittedAt)} に提出`;
}

// 最初に失敗したテストケース。無いときは null。
export function firstFailedCase(result: SubmitResult): CaseResult | null {
  return result.cases.find((item) => item.status !== "pass") ?? null;
}

export function isStdoutFailDetail(
  detail: CaseResult["detail"],
): detail is StdoutFailDetail {
  return typeof detail === "object" && detail !== null;
}

// 空白を見えるようにした文字列の断片。ws が true の断片は、␣ に置き換えた空白。
export type TextPart = { text: string; ws: boolean };

// 行頭と行末の空白（スペースとタブ）を ␣ に置き換えて、見えるようにする。行の途中の空白は、そのままにする。
// 改行コードの CR は、行末の空白として扱う。
export function visibleWhitespace(text: string): TextPart[] {
  const parts: TextPart[] = [];
  const push = (part: string, ws: boolean) => {
    if (part === "") {
      return;
    }
    const last = parts[parts.length - 1];
    if (last !== undefined && last.ws === ws) {
      last.text += part;
    } else {
      parts.push({ text: part, ws });
    }
  };
  text.split("\n").forEach((line, index) => {
    if (index > 0) {
      push("\n", false);
    }
    const match = /^([ \t]*)(.*?)([ \t\r]*)$/s.exec(line);
    const [, head, body, tail] = match ?? ["", "", line, ""];
    push("␣".repeat(head.length), true);
    push(body, false);
    push("␣".repeat(tail.length), true);
  });
  return parts;
}
