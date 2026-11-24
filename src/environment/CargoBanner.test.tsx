import { act, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, expect, it } from "vitest";
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

it("not_found のときは rustup の導入手順を案内する", () => {
  render(<CargoBanner cargo={NOT_FOUND} />);
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Rust ツールチェーンが見つかりません",
  );
  expect(screen.getByRole("alert")).toHaveTextContent("https://rustup.rs");
});

it("version_too_old のときは今と必要なバージョンと更新の手順を案内する", () => {
  render(<CargoBanner cargo={TOO_OLD} />);
  const alert = screen.getByRole("alert");
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
