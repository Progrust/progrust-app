import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { applyTheme, resetTheme, resolveTheme, startTheme } from "./theme";

// OS の設定を置き換える。setDark で OS の設定を変え、変更の通知を出す。
function stubOsTheme(initialDark: boolean) {
  let dark = initialDark;
  const listeners = new Set<() => void>();
  const query = {
    get matches() {
      return dark;
    },
    addEventListener: (_: string, listener: () => void) =>
      listeners.add(listener),
    removeEventListener: (_: string, listener: () => void) =>
      listeners.delete(listener),
  };
  vi.stubGlobal("matchMedia", () => query);
  return {
    setDark(value: boolean) {
      dark = value;
      listeners.forEach((listener) => {
        listener();
      });
    },
  };
}

const themeOf = () => document.documentElement.dataset.theme;

beforeEach(() => {
  resetTheme();
});

afterEach(() => {
  resetTheme();
  vi.unstubAllGlobals();
});

it("resolveTheme は system のとき OS の設定に従う", () => {
  expect(resolveTheme("system", true)).toBe("dark");
  expect(resolveTheme("system", false)).toBe("light");
  expect(resolveTheme("light", true)).toBe("light");
  expect(resolveTheme("dark", false)).toBe("dark");
});

it("system のとき、OS の設定が変わるとテーマが切り替わる", () => {
  const os = stubOsTheme(false);
  applyTheme("system");
  expect(themeOf()).toBe("light");
  os.setDark(true);
  expect(themeOf()).toBe("dark");
  os.setDark(false);
  expect(themeOf()).toBe("light");
});

it("light・dark のとき、OS の設定が変わっても切り替わらない", () => {
  const os = stubOsTheme(false);
  applyTheme("dark");
  os.setDark(false);
  os.setDark(true);
  expect(themeOf()).toBe("dark");
  applyTheme("light");
  expect(themeOf()).toBe("light");
});

it("設定を変えるとすぐ反映される", () => {
  stubOsTheme(true);
  applyTheme("system");
  expect(themeOf()).toBe("dark");
  applyTheme("light");
  expect(themeOf()).toBe("light");
});

it("startTheme は get_settings の theme を反映する", async () => {
  stubOsTheme(false);
  await startTheme(() => Promise.resolve({ theme: "dark" }));
  expect(themeOf()).toBe("dark");
});

it("startTheme は get_settings に失敗したら OS の設定に合わせる", async () => {
  stubOsTheme(true);
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  await startTheme(() => Promise.reject(new Error("失敗")));
  expect(themeOf()).toBe("dark");
  expect(error).toHaveBeenCalled();
});
