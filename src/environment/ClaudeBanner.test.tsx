import { act, fireEvent, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, it, vi } from "vitest";
import type { ClaudeStatus } from "../commands/environment";
import { NavigationContext } from "../router";
import type { Route } from "../router";
import { ClaudeBanner } from "./ClaudeBanner";

afterEach(() => {
  clearMocks();
});

const NOT_FOUND: ClaudeStatus = {
  status: "not_found",
  path: null,
  version: null,
};

function renderBanner(claude: ClaudeStatus | null, routes: Route[] = []) {
  return render(
    <NavigationContext.Provider
      value={{
        route: { name: "home" },
        navigate: (route) => routes.push(route),
      }}
    >
      <ClaudeBanner claude={claude} />
    </NavigationContext.Provider>,
  );
}

it("found・読み込み前は案内を出さない", () => {
  const found: ClaudeStatus = { status: "found", path: "/c", version: "1" };
  expect(renderBanner(found).container).toBeEmptyDOMElement();
  expect(renderBanner(null).container).toBeEmptyDOMElement();
});

it("not_found のときは案内を出し、導入手順を開く", async () => {
  const opened: unknown[] = [];
  mockIPC((command, args) => {
    if (command === "plugin:opener|open_url") {
      opened.push(args);
    }
    return null;
  });
  renderBanner(NOT_FOUND);
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Claude Code が見つかりません",
  );
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: /導入手順を見る/ }));
  });
  expect(opened).toEqual([
    { url: "https://code.claude.com/docs/ja/setup", with: undefined },
  ]);
});

it("「パスを指定」で設定画面に移る", () => {
  const routes: Route[] = [];
  renderBanner(NOT_FOUND, routes);
  fireEvent.click(screen.getByRole("button", { name: "パスを指定" }));
  expect(routes).toEqual([{ name: "settings" }]);
});

it("導入手順を開けなくても、案内は出たままにする", async () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  mockIPC(() => {
    throw new Error("開けない");
  });
  renderBanner(NOT_FOUND);
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: /導入手順を見る/ }));
  });
  expect(screen.getByRole("alert")).toBeInTheDocument();
  expect(error).toHaveBeenCalled();
  error.mockRestore();
});
