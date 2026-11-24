import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, it } from "vitest";
import capability from "../../src-tauri/capabilities/default.json";
import { INSTALL_GUIDE_URLS, openInstallGuide } from "./installGuide";

afterEach(() => {
  clearMocks();
});

it("導入手順の URL は、仕様の表の 2 つである", () => {
  expect(INSTALL_GUIDE_URLS).toEqual({
    cargo: "https://rustup.rs",
    claude: "https://code.claude.com/docs/ja/setup",
  });
});

// 権限で開ける URL とずれると、ボタンを押しても開けない（権限が拒否する）。
it("導入手順の URL は、権限で開ける URL と同じである", () => {
  const scoped = capability.permissions.filter(
    (permission) => typeof permission !== "string",
  );
  expect(scoped.map((permission) => permission.identifier)).toEqual([
    "opener:allow-open-url",
  ]);
  expect(scoped[0].allow.map((entry) => entry.url).sort()).toEqual(
    Object.values(INSTALL_GUIDE_URLS).sort(),
  );
});

it.each([
  ["cargo", "https://rustup.rs"],
  ["claude", "https://code.claude.com/docs/ja/setup"],
] as const)("%s の導入手順を、%s で開く", async (tool, url) => {
  const calls: { command: string; args: unknown }[] = [];
  mockIPC((command, args) => {
    calls.push({ command, args });
    return null;
  });
  await openInstallGuide(tool);
  expect(calls).toEqual([
    { command: "plugin:opener|open_url", args: { url, with: undefined } },
  ]);
});
