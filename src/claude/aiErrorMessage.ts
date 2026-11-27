// AI 呼び出しのエラーの `category` から、画面に出す文を決める
// （documents/spec/claude-integration/errors.md「分類」、
// documents/spec/architecture/commands/conventions.md「AI 呼び出しのエラー」）。
// 質疑応答の画面と、設定画面の「接続テスト」が使う。
export type AiErrorCategory =
  "not_logged_in" | "rate_limited" | "timeout" | "other";

// AI 呼び出しのエラー。`detail` は「その他」だけにある。
export type AiError = {
  category: AiErrorCategory;
  detail: string | null;
};

export type AiErrorMessage = {
  // 画面に出す文。
  message: string;
  // 折りたたんで出す詳細。「その他」で、詳細があるときだけ。
  detail: string | null;
};

export function aiErrorMessage(error: AiError): AiErrorMessage {
  switch (error.category) {
    case "not_logged_in":
      return {
        message: "ターミナルで `claude` を起動してログインしてください",
        detail: null,
      };
    case "rate_limited":
      return {
        message:
          "Claude の利用上限に達しました。時間をおいて再試行してください",
        detail: null,
      };
    case "timeout":
      return { message: "応答がありませんでした", detail: null };
    case "other": {
      const detail = error.detail?.trim() ?? "";
      return {
        message: "AI の呼び出しに失敗しました",
        detail: detail === "" ? null : detail,
      };
    }
  }
}
