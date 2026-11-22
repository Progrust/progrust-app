import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { App } from "./App";

// バックエンドの呼び出しの記録。
let calls: { cmd: string; args: unknown }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === "get_app_version") {
      return { version: "9.8.7" };
    }
    throw new Error(`想定していないコマンド: ${cmd}`);
  });
});

afterEach(() => {
  clearMocks();
});

it("仮のページを表示する", async () => {
  render(<App />);
  expect(
    screen.getByRole("heading", { level: 1, name: "Progrust" }),
  ).toBeInTheDocument();
  // 受け取りを待ち、テストの後に状態が変わらないようにする。
  expect(await screen.findByText("バージョン 9.8.7")).toBeInTheDocument();
});

it("get_app_version で受け取ったバージョンを表示する", async () => {
  render(<App />);
  expect(await screen.findByText("バージョン 9.8.7")).toBeInTheDocument();
  expect(calls.map((call) => call.cmd)).toContain("get_app_version");
  // 引数の無いコマンドは、引数を渡さずに呼ぶ。
  expect(calls.find((call) => call.cmd === "get_app_version")?.args).toEqual(
    {},
  );
});
