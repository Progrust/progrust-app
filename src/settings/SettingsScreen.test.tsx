import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import type { Settings } from "../commands/settings";
import { resetTheme } from "../theme";
import { SettingsScreen } from "./SettingsScreen";

let stored: Settings;
let saved: Settings[];

beforeEach(() => {
  resetTheme();
  saved = [];
  stored = {
    schema_version: 1,
    theme: "system",
    editor_font_size: 14,
    video: { volume: 1, muted: false, playback_rate: 1, captions: false },
    models: { chat: "sonnet", background: "opus" },
    tool_paths: { cargo: null, claude: null },
    first_launch_completed: true,
  };
  mockIPC((command, args) => {
    if (command === "get_settings") {
      return stored;
    }
    if (command === "get_app_version") {
      return { version: "0.1.0" };
    }
    if (command === "update_settings") {
      stored = (args as { settings: Settings }).settings;
      saved.push(stored);
      return stored;
    }
    return undefined;
  });
});

afterEach(() => {
  clearMocks();
  resetTheme();
});

it("バージョンと、現在の設定の値を表示する", async () => {
  render(<SettingsScreen />);
  expect(await screen.findByText("Progrust 0.1.0")).toBeInTheDocument();
  expect(
    screen.getByRole("radio", { name: "システムに合わせる" }),
  ).toBeChecked();
  expect(
    screen.getByRole("combobox", { name: "エディタの文字サイズ" }),
  ).toHaveValue("14");
  expect(screen.getByRole("radio", { name: "1 倍" })).toBeChecked();
  expect(screen.getByRole("switch", { name: "表示する" })).not.toBeChecked();
  expect(screen.getByRole("slider", { name: "音量" })).toHaveAttribute(
    "aria-valuenow",
    "100",
  );
});

it("設定を変えると update_settings で保存され、開き直しても変えた値が表示される", async () => {
  const first = render(<SettingsScreen />);
  fireEvent.click(await screen.findByRole("radio", { name: "ダーク" }));
  fireEvent.change(
    screen.getByRole("combobox", { name: "エディタの文字サイズ" }),
    {
      target: { value: "20" },
    },
  );
  fireEvent.click(screen.getByRole("radio", { name: "1.5 倍" }));
  fireEvent.click(screen.getByRole("switch", { name: "表示する" }));
  fireEvent.click(screen.getByRole("button", { name: "ミュート" }));
  fireEvent.keyDown(screen.getByRole("slider", { name: "音量" }), {
    key: "ArrowLeft",
  });
  await waitFor(() => {
    expect(saved).toHaveLength(6);
  });
  expect(stored).toMatchObject({
    theme: "dark",
    editor_font_size: 20,
    video: { volume: 0.95, muted: true, playback_rate: 1.5, captions: true },
    // 変えていない項目は、そのまま保存する。
    models: { chat: "sonnet", background: "opus" },
    first_launch_completed: true,
  });

  first.unmount();
  render(<SettingsScreen />);
  expect(await screen.findByRole("radio", { name: "ダーク" })).toBeChecked();
  expect(
    screen.getByRole("combobox", { name: "エディタの文字サイズ" }),
  ).toHaveValue("20");
  expect(screen.getByRole("radio", { name: "1.5 倍" })).toBeChecked();
  expect(screen.getByRole("switch", { name: "表示する" })).toBeChecked();
  expect(screen.getByRole("button", { name: "ミュート" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  expect(screen.getByRole("slider", { name: "音量" })).toHaveAttribute(
    "aria-valuenow",
    "95",
  );
});

it("テーマを変えると、保存を待たずに、ルート要素の data-theme がすぐ切り替わる", async () => {
  render(<SettingsScreen />);
  fireEvent.click(await screen.findByRole("radio", { name: "ダーク" }));
  expect(document.documentElement.dataset.theme).toBe("dark");
  fireEvent.click(screen.getByRole("radio", { name: "ライト" }));
  expect(document.documentElement.dataset.theme).toBe("light");
  await waitFor(() => {
    expect(saved).toHaveLength(2);
  });
});

it("保存に失敗したら、そのことを表示する", async () => {
  mockIPC((command) => {
    if (command === "get_settings") {
      return stored;
    }
    if (command === "get_app_version") {
      return { version: "0.1.0" };
    }
    throw new Error("失敗");
  });
  render(<SettingsScreen />);
  fireEvent.click(await screen.findByRole("switch", { name: "表示する" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "設定を保存できませんでした。",
  );
});
