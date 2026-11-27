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
