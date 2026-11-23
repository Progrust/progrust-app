import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it } from "vitest";
import { App } from "./App";

const MENU_LABELS = ["ホーム", "レッスン", "問題生成", "設定"];

function menu() {
  return within(screen.getByRole("navigation", { name: "メインメニュー" }));
}

it("ヘッダーにロゴとメインメニューを表示し、最初はホームを選ぶ", () => {
  render(<App />);
  const header = screen.getByRole("banner");
  expect(within(header).getByText("Progrust")).toBeInTheDocument();
  expect(
    menu()
      .getAllByRole("link")
      .map((link) => link.textContent),
  ).toEqual(MENU_LABELS);
  expect(menu().getByRole("link", { name: "ホーム" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
    "トップページ",
  );
});

it("メニューで各画面に移り、選んでいる項目だけに aria-current を付ける", () => {
  render(<App />);
  const cases = [
    ["レッスン", "レッスン一覧"],
    ["問題生成", "生成問題一覧"],
    ["設定", "設定画面"],
    ["ホーム", "トップページ"],
  ] as const;
  for (const [label, title] of cases) {
    fireEvent.click(menu().getByRole("link", { name: label }));
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(title);
    // ヘッダーは、どの画面でも表示される。
    expect(screen.getByRole("banner")).toBeInTheDocument();
    const current = menu()
      .getAllByRole("link")
      .filter((link) => link.getAttribute("aria-current") === "page");
    expect(current.map((link) => link.textContent)).toEqual([label]);
  }
});

it("生成問題一覧は準備中と表示する", () => {
  render(<App />);
  fireEvent.click(menu().getByRole("link", { name: "問題生成" }));
  expect(screen.getByText("準備中")).toBeInTheDocument();
});
