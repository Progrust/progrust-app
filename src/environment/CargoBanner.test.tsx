import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, expect, it, vi } from "vitest";
import type { CargoStatus, EnvironmentStatus } from "../commands/environment";
import { CargoBanner } from "./CargoBanner";
import { cargoUsable, useEnvironmentStatus } from "./useEnvironmentStatus";

afterEach(() => {
  clearMocks();
});

function cargo(overrides: Partial<CargoStatus>): CargoStatus {
  return {
    status: "found",
    cargo_path: "/c/cargo",
    rustc_path: "/c/rustc",
    rustc_version: "1.100.0",
    required_rustc_version: "1.100.0",
    ...overrides,
  };
}

function env(c: CargoStatus): EnvironmentStatus {
  return {
    cargo: c,
    claude: { status: "not_found", path: null, version: null },
    crates: { status: "ready", error: null },
  };
}

const NOT_FOUND = cargo({
  status: "not_found",
  cargo_path: null,
  rustc_path: null,
  rustc_version: null,
});
const TOO_OLD = cargo({ status: "version_too_old", rustc_version: "1.99.0" });

it("found のときは案内を出さない", () => {
  const { container } = render(<CargoBanner cargo={cargo({})} />);
  expect(container).toBeEmptyDOMElement();
});

it("not_found のときは rustup の導入手順を案内し、ボタンで https://rustup.rs を開く", async () => {
  const opened: unknown[] = [];
  mockIPC((command, args) => {
    if (command === "plugin:opener|open_url") {
      opened.push(args);
    }
    return null;
  });
  render(<CargoBanner cargo={NOT_FOUND} />);
  const alert = screen.getByRole("alert");
  expect(alert).toHaveTextContent("Rust ツールチェーンが見つかりません");
  expect(alert).toHaveTextContent("アプリを開き直してください");
  expect(alert).not.toHaveTextContent("https://");
  await act(async () => {
    fireEvent.click(
      within(alert).getByRole("button", { name: "rustup の導入手順" }),
    );
  });
  expect(opened).toEqual([{ url: "https://rustup.rs", with: undefined }]);
});

it("導入手順を開けなくても、案内は出たままにする", async () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  mockIPC(() => {
    throw new Error("開けない");
  });
  render(<CargoBanner cargo={NOT_FOUND} />);
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "rustup の導入手順" }));
  });
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Rust ツールチェーンが見つかりません",
  );
  expect(error).toHaveBeenCalled();
  error.mockRestore();
});

it("version_too_old のときは今と必要なバージョンと更新の手順を案内する（導入手順のボタンは出さない）", () => {
  render(<CargoBanner cargo={TOO_OLD} />);
  const alert = screen.getByRole("alert");
  expect(within(alert).queryByRole("button")).toBeNull();
  expect(alert).toHaveTextContent("1.99.0");
  expect(alert).toHaveTextContent("1.100.0");
  expect(alert).toHaveTextContent("rustup update stable");
});

it("not_found のときだけ、実行・テスト・提出を無効にする", () => {
  expect(cargoUsable(null)).toBe(true);
  expect(cargoUsable(env(cargo({})))).toBe(true);
  expect(cargoUsable(env(TOO_OLD))).toBe(true);
  expect(cargoUsable(env(NOT_FOUND))).toBe(false);
});

function Probe() {
  const status = useEnvironmentStatus();
  return <p>{status === null ? "読み込み中" : status.cargo.status}</p>;
}

it("環境の状態を読み、environment_updated で更新する", async () => {
  mockIPC(
    (command) => (command === "get_environment_status" ? env(NOT_FOUND) : null),
    { shouldMockEvents: true },
  );
  render(<Probe />);
  expect(await screen.findByText("not_found")).toBeInTheDocument();
  await act(async () => {
    await emit("environment_updated", env(cargo({})));
  });
  expect(await screen.findByText("found")).toBeInTheDocument();
});

it("環境の状態を読めなくても、案内を出さず使える", async () => {
  mockIPC(
    (command) => {
      if (command === "get_environment_status") {
        throw new Error("失敗");
      }
      return null;
    },
    { shouldMockEvents: true },
  );
  render(<Probe />);
  await act(async () => {});
  expect(screen.getByText("読み込み中")).toBeInTheDocument();
});
