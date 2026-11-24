// 演習画面がコンソールに書く行（区切りの行、提出の経過、終わりの知らせ）と、見出しの行の進み具合の文
// （design-system/ui-design/Exercise.dc.html のコンソール、design-system/progrust/components/Console/README.md）。
// 画面の組み立てから分けて、自動テストで確かめられるようにする。
import type { AnsiStyle } from "../console/ansi";
import type { RunFinished, RunMode, SubmitProgress } from "../commands/run";

// コンソールに書く 1 行。system は muted の色、progress は既定の色で書く。
export type ConsoleLine = {
  kind: "system" | "progress";
  text: string;
  style?: AnsiStyle;
};

export type StartKind = { kind: "run"; mode: RunMode } | { kind: "submit" };

function startLabel(start: StartKind): string {
  if (start.kind === "submit") {
    return "提出";
  }
  return start.mode === "main" ? "実行" : "テスト";
}

// 始めたときの区切りの行。例: 「── 実行  14:30 ──」。時刻は、ボタンを押したときの、手元の時刻の時と分。
export function separatorLine(start: StartKind, at: Date): string {
  const hh = String(at.getHours()).padStart(2, "0");
  const mm = String(at.getMinutes()).padStart(2, "0");
  return `── ${startLabel(start)}  ${hh}:${mm} ──`;
}

// 提出で、ビルドが終わってテストケースの実行に移ったときの行。
export const BUILT_LINE = "ビルドしました";

// run_finished を受け取ったときに書く行。タイムアウトと出力量の上限と crates_unavailable は、バックエンドが
// run_output の system で知らせるので、ここでは書かない。判定結果のカードは別に出す。
export function finishedLines(event: RunFinished): ConsoleLine[] {
  if (event.error !== null) {
    return [{ kind: "system", text: event.error.message }];
  }
  if (event.run !== null) {
    const { mode, status, exit_code } = event.run;
    switch (status) {
      case "compile_error":
        return [
          {
            kind: "system",
            text:
              mode === "main"
                ? "ビルドに失敗したため、実行していません。"
                : "ビルドに失敗したため、テストを実行していません。",
          },
        ];
      case "success":
      case "runtime_error":
        return [
          {
            kind: "system",
            text:
              exit_code === null
                ? "終了しました"
                : `終了しました（終了コード ${exit_code}）`,
          },
        ];
      case "stopped":
        return [{ kind: "system", text: "停止しました" }];
      case "timeout":
      case "crates_unavailable":
        return [];
    }
  }
  if (event.submit !== null) {
    const { status, passed, total } = event.submit;
    const ran: ConsoleLine = {
      kind: "progress",
      text: `テストを実行しました（${total} / ${total}）`,
    };
    switch (status) {
      case "compile_error":
        return [
          {
            kind: "system",
            text: "ビルドに失敗したため、テストを実行していません。",
          },
        ];
      case "passed":
        return [
          ran,
          {
            kind: "progress",
            text: `${total} 件のテストがすべて通りました。`,
            style: { color: null, bold: true },
          },
        ];
      case "wrong":
        return [
          ran,
          {
            kind: "progress",
            text: `${total} 件中 ${total - passed} 件のテストが失敗しました。`,
            style: { color: "red", bold: true },
          },
        ];
      case "stopped":
      case "crates_unavailable":
        return [];
    }
  }
  return [];
}

// 見出しの行に、回るアイコンと一緒に出す文。
// progress は、提出で最後に受け取った submit_progress（まだ受け取っていなければ null）。
export function busyText(
  state: "running" | "testing" | "submitting",
  progress: SubmitProgress | null,
): string {
  switch (state) {
    case "running":
      return "実行中";
    case "testing":
      return "テスト実行中";
    case "submitting":
      if (progress === null || progress.phase === "build") {
        return "ビルド中";
      }
      return progress.total === null
        ? "判定中"
        : `判定中 ${progress.completed} / ${progress.total}`;
  }
}
