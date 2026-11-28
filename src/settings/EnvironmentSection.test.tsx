import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { EnvironmentStatus } from "../commands/environment";
import type { Settings } from "../commands/settings";
import { resetTheme } from "../theme";
import { formatPreparedAt } from "./EnvironmentSection";
import { SettingsScreen } from "./SettingsScreen";

let stored: Settings;
let environment: EnvironmentStatus;
// update_settings・redetect_tools・retry_crate_preparation が呼ばれた順番。
let calls: string[];
let opened: unknown[];

const FOUND: EnvironmentStatus = {
  cargo: {
    status: "found",
    cargo_path: "/home/me/.cargo/bin/cargo",
    rustc_path: "/home/me/.cargo/bin/rustc",
    rustc_version: "1.100.0",
    required_rustc_version: "1.100.0",
  },
  claude: {
    status: "found",
    path: "/home/me/.local/bin/claude",
    version: "2.1.329",
  },
  crates: {
    status: "ready",
    prepared_at: "2026-11-24T19:02:00+09:00",
    error: null,
  },
};

beforeEach(() => {
  resetTheme();
  calls = [];
  opened = [];
  environment = FOUND;
  stored = {
    schema_version: 2,
    theme: "system",
    editor_font_size: 14,
    video: { volume: 1, muted: false, playback_rate: 1, captions: false },
    models: { chat: "sonnet", background: "opus" },
    tool_paths: { cargo: null, claude: null, clang: null },
    first_launch_completed: true,
  };
  mockIPC(
    (command, args) => {
      switch (command) {
        case "get_settings":
          return stored;
        case "get_app_version":
          return { version: "0.1.0" };
        case "get_environment_status":
          return environment;
        case "update_settings":
          calls.push(
            `update_settings:${JSON.stringify(
              (args as { settings: Settings }).settings.tool_paths,
            )}`,
          );
          stored = (args as { settings: Settings }).settings;
          return stored;
        case "redetect_tools":
          calls.push("redetect_tools");
          return environment;
        case "retry_crate_preparation":
          calls.push("retry_crate_preparation");
          return null;
        case "plugin:opener|open_url":
          opened.push(args);
          return null;
        default:
          return undefined;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  clearMocks();
  resetTheme();
});

async function envSection() {
  const heading = await screen.findByRole("heading", { name: "環境" });
  const section = heading.closest("section");
  if (section === null) {
    throw new Error("環境の区分が無い");
  }
  return within(section);
}

function row(name: string) {
  const heading = screen.getByRole("heading", { name });
  const element = heading.closest(".pg-settings-row");
  if (!(element instanceof HTMLElement)) {
    throw new Error(`${name} の行が無い`);
  }
  return within(element);
}

it("cargo・claude の検出したパスとバージョンと、自動で検出したことを表示する", async () => {
  render(<SettingsScreen />);
  await envSection();
  expect(
    await screen.findByText("/home/me/.cargo/bin/cargo"),
  ).toBeInTheDocument();
  const cargo = screen.getByRole("heading", {
    name: "Rust ツールチェーン",
  }).parentElement;
  expect(cargo).toHaveTextContent("rustc 1.100.0");
  expect(cargo).toHaveTextContent("（自動で検出）");
  const claude = screen.getByRole("heading", {
    name: "Claude Code",
  }).parentElement;
  expect(claude).toHaveTextContent("2.1.329");
  expect(claude).toHaveTextContent("/home/me/.local/bin/claude");
  expect(claude).toHaveTextContent("（自動で検出）");
  expect(screen.queryByRole("alert")).toBeNull();
});

it("設定で指定したパスが使われているときは、指定したパスと表示する", async () => {
  stored.tool_paths.cargo = "/home/me/.cargo/bin/cargo";
  render(<SettingsScreen />);
  const cargo = (
    await screen.findByRole("heading", {
      name: "Rust ツールチェーン",
    })
  ).parentElement;
  await waitFor(() => {
    expect(cargo).toHaveTextContent("（指定したパス）");
  });
  expect(screen.queryByRole("alert")).toBeNull();
});

it("見つからないときは、そのことと導入手順のボタンを表示し、ボタンで導入手順を開く", async () => {
  environment = {
    ...FOUND,
    cargo: {
      status: "not_found",
      cargo_path: null,
      rustc_path: null,
      rustc_version: null,
      required_rustc_version: "1.100.0",
    },
    claude: { status: "not_found", path: null, version: null },
    crates: { status: "not_started", prepared_at: null, error: null },
  };
  render(<SettingsScreen />);
  const cargoRow = await waitFor(() => {
    const r = row("Rust ツールチェーン");
    r.getByText("見つかりませんでした。");
    return r;
  });
  row("Claude Code").getByText("見つかりませんでした。");
  expect(
    screen.getByText(/Rust ツールチェーンが見つからないので、準備していません/),
  ).toBeInTheDocument();
  await act(async () => {
    fireEvent.click(cargoRow.getByRole("button", { name: "導入手順を見る" }));
  });
  expect(opened).toEqual([{ url: "https://rustup.rs", with: undefined }]);
  await act(async () => {
    fireEvent.click(
      row("Claude Code").getByRole("button", { name: "導入手順を見る" }),
    );
  });
  expect(opened[1]).toEqual({
    url: "https://code.claude.com/docs/ja/setup",
    with: undefined,
  });
});

it("rustc のバージョンが古いときは、今のバージョンと必要なバージョンを表示する", async () => {
  environment = {
    ...FOUND,
    cargo: {
      ...FOUND.cargo,
      status: "version_too_old",
      rustc_version: "1.99.0",
    },
  };
  render(<SettingsScreen />);
  const cargo = (
    await screen.findByRole("heading", {
      name: "Rust ツールチェーン",
    })
  ).parentElement;
  await waitFor(() => {
    expect(cargo).toHaveTextContent("1.99.0");
  });
  expect(cargo).toHaveTextContent("必要なバージョンは 1.100.0");
  expect(cargo).toHaveTextContent("/home/me/.cargo/bin/cargo");
});

it("パスを入力して保存すると、tool_paths を保存してから redetect_tools を呼ぶ", async () => {
  render(<SettingsScreen />);
  await screen.findByText("/home/me/.cargo/bin/cargo");
  fireEvent.click(
    screen.getByRole("button", { name: "Rust ツールチェーンのパスを指定" }),
  );
  fireEvent.change(
    screen.getByRole("textbox", { name: "Rust ツールチェーンのパス" }),
    {
      target: { value: "  /opt/rust/bin/cargo  " },
    },
  );
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => {
    expect(calls).toEqual([
      'update_settings:{"cargo":"/opt/rust/bin/cargo","claude":null,"clang":null}',
      "redetect_tools",
    ]);
  });
  // 保存できたら、入力欄を閉じる。
  await waitFor(() => {
    expect(
      screen.queryByRole("textbox", { name: "Rust ツールチェーンのパス" }),
    ).toBeNull();
  });
});

it("入力欄を空にして保存すると、null（自動検出）に戻す", async () => {
  stored.tool_paths.claude = "/x/claude";
  render(<SettingsScreen />);
  await screen.findByText("/home/me/.local/bin/claude");
  fireEvent.click(
    screen.getByRole("button", { name: "Claude Codeのパスを指定" }),
  );
  const input = screen.getByRole("textbox", { name: "Claude Codeのパス" });
  expect(input).toHaveValue("/x/claude");
  fireEvent.change(input, { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => {
    expect(calls).toEqual([
      'update_settings:{"cargo":null,"claude":null,"clang":null}',
      "redetect_tools",
    ]);
  });
});

it("指定したパスが使われなかったときは、指定したパスが使えなかったことを表示する", async () => {
  stored.tool_paths.cargo = "/nowhere/cargo";
  render(<SettingsScreen />);
  const alert = await screen.findByRole("alert");
  expect(alert).toHaveTextContent("/nowhere/cargo");
  expect(alert).toHaveTextContent("使えませんでした");
  // ほかの場所で見つかったので、見つかったパスも表示する。
  expect(screen.getByText("/home/me/.cargo/bin/cargo")).toBeInTheDocument();
  expect(screen.getAllByRole("alert")).toHaveLength(1);
});

it("指定したパスで見つからなかったときも、指定したパスが使えなかったことを表示する", async () => {
  stored.tool_paths.claude = "/nowhere/claude";
  environment = {
    ...FOUND,
    claude: { status: "not_found", path: null, version: null },
  };
  render(<SettingsScreen />);
  const alert = await screen.findByRole("alert");
  expect(alert).toHaveTextContent("/nowhere/claude");
  expect(alert).toHaveTextContent("使えませんでした");
});

it("パスを保存できなかったときは、検出し直さず、そのことを表示する", async () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  mockIPC((command) => {
    if (command === "get_settings") {
      return stored;
    }
    if (command === "get_app_version") {
      return { version: "0.1.0" };
    }
    if (command === "get_environment_status") {
      return environment;
    }
    if (command === "update_settings") {
      throw new Error("失敗");
    }
    calls.push(command);
    return undefined;
  });
  render(<SettingsScreen />);
  await screen.findByText("/home/me/.cargo/bin/cargo");
  fireEvent.click(
    screen.getByRole("button", { name: "Rust ツールチェーンのパスを指定" }),
  );
  fireEvent.change(
    screen.getByRole("textbox", { name: "Rust ツールチェーンのパス" }),
    {
      target: { value: "/x/cargo" },
    },
  );
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "保存できませんでした。",
  );
  expect(calls).not.toContain("redetect_tools");
  error.mockRestore();
});

it("「再検出」で redetect_tools を呼ぶ", async () => {
  render(<SettingsScreen />);
  await screen.findByText("/home/me/.cargo/bin/cargo");
  fireEvent.click(screen.getByRole("button", { name: "Claude Codeを再検出" }));
  await waitFor(() => {
    expect(calls).toEqual(["redetect_tools"]);
  });
});

it("environment_updated で表示が更新される", async () => {
  environment = {
    ...FOUND,
    claude: { status: "not_found", path: null, version: null },
  };
  render(<SettingsScreen />);
  await waitFor(() => {
    row("Claude Code").getByText("見つかりませんでした。");
  });
  await act(async () => {
    await emit("environment_updated", FOUND);
  });
  expect(
    await screen.findByText("/home/me/.local/bin/claude"),
  ).toBeInTheDocument();
});

it("外部クレートの準備が ready のときは、準備できた日時を表示する", async () => {
  render(<SettingsScreen />);
  const crates = await waitFor(() => row("外部クレートの準備"));
  await waitFor(() => {
    crates.getByText(/準備できています/);
  });
  expect(crates.getByText(/準備できています/)).toHaveTextContent(
    `（${formatPreparedAt(FOUND.crates.prepared_at ?? "")}）`,
  );
  expect(crates.queryByRole("button", { name: "再試行" })).toBeNull();
});

it("formatPreparedAt は「月/日 時:分」にし、読めない日時はそのまま返す", () => {
  const date = new Date(2026, 9, 28, 9, 5);
  expect(formatPreparedAt(date.toISOString())).toBe("10/28 09:05");
  expect(formatPreparedAt("きのう")).toBe("きのう");
});

it("外部クレートの準備が preparing のときは、準備中と表示する", async () => {
  environment = {
    ...FOUND,
    crates: { status: "preparing", prepared_at: null, error: null },
  };
  render(<SettingsScreen />);
  await waitFor(() => {
    row("外部クレートの準備").getByText(/準備中です/);
  });
});

it("外部クレートの準備が failed のときは、エラーの内容と再試行のボタンを表示し、押すと retry_crate_preparation を呼ぶ", async () => {
  environment = {
    ...FOUND,
    crates: {
      status: "failed",
      prepared_at: null,
      error: "ネットワークに接続できません",
    },
  };
  render(<SettingsScreen />);
  const crates = await waitFor(() => {
    const r = row("外部クレートの準備");
    r.getByRole("button", { name: "再試行" });
    return r;
  });
  expect(screen.getByText(/ネットワークに接続できません/)).toBeInTheDocument();
  fireEvent.click(crates.getByRole("button", { name: "再試行" }));
  await waitFor(() => {
    expect(calls).toEqual(["retry_crate_preparation"]);
  });
});
