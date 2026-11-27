import { expect, it } from "vitest";
import { aiErrorMessage } from "./aiErrorMessage";

it("分類ごとに、errors.md の文を返す", () => {
  expect(aiErrorMessage({ category: "not_logged_in", detail: null })).toEqual({
    message: "ターミナルで `claude` を起動してログインしてください",
    detail: null,
  });
  expect(aiErrorMessage({ category: "rate_limited", detail: null })).toEqual({
    message: "Claude の利用上限に達しました。時間をおいて再試行してください",
    detail: null,
  });
  expect(aiErrorMessage({ category: "timeout", detail: null })).toEqual({
    message: "応答がありませんでした",
    detail: null,
  });
});

it("「その他」は固定の文と、詳細を返す", () => {
  expect(
    aiErrorMessage({ category: "other", detail: "model not found" }),
  ).toEqual({
    message: "AI の呼び出しに失敗しました",
    detail: "model not found",
  });
});

it("「その他」で詳細が無い・空のときは、詳細を出さない", () => {
  for (const detail of [null, "", "  \n"]) {
    expect(aiErrorMessage({ category: "other", detail }).detail).toBeNull();
  }
});
