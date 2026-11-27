// コマンドが失敗したときに返るエラー（documents/spec/architecture/commands/conventions.md「エラー」）。
export type CommandErrorCode =
  | "invalid_argument"
  | "not_found"
  | "invalid_state"
  | "busy"
  | "cargo_unavailable"
  | "claude_unavailable"
  | "crates_not_ready"
  | "io_error"
  | "internal";

export type CommandError = {
  code: CommandErrorCode;
  message: string;
  detail: string | null;
};

// `invoke` が拒否した値が、コマンドのエラーの形なら、その値を返す。そうでなければ `null`。
export function asCommandError(value: unknown): CommandError | null {
  if (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    "message" in value &&
    typeof value.message === "string"
  ) {
    return value as CommandError;
  }
  return null;
}

// コマンドが拒否されたときに、画面に出す文。コマンドのエラー（CommandError）の message（学習者に見せてよい日本語）を
// 使う。CommandError でないもの（コマンドを呼べなかったときの Error など）は、決まった文にする。
export function commandErrorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    !(error instanceof Error) &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message !== ""
  ) {
    return error.message;
  }
  return "コマンドを実行できませんでした。";
}
