import { act, fireEvent, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, it, vi } from "vitest";
import type { CratesStatus, EnvironmentStatus } from "../commands/environment";
import { CratesBanner } from "./CratesBanner";
import { cratesReadyFor } from "./useEnvironmentStatus";

afterEach(() => {
  clearMocks();
});

function crates(status: CratesStatus["status"]): CratesStatus {
  return { status, prepared_at: null, error: null };
}

function env(status: CratesStatus["status"]): EnvironmentStatus {
  return {
    cargo: {
      status: "found",
      cargo_path: "/c/cargo",
      rustc_path: "/c/rustc",
      rustc_version: "1.100.0",
      required_rustc_version: "1.100.0",
    },
    claude: { status: "found", path: "/c/claude", version: "1.0.0" },
    crates: crates(status),
  };
}

it("not_started・ready・読み込み前は案内を出さない", () => {
  for (const status of ["not_started", "ready"] as const) {
    const { container } = render(
      <CratesBanner crates={crates(status)} where="home" />,
    );
    expect(container).toBeEmptyDOMElement();
  }
  const { container } = render(<CratesBanner crates={null} where="home" />);
  expect(container).toBeEmptyDOMElement();
});

it("preparing のときは「外部クレートを準備しています」を出し、再試行のボタンは出さない", () => {
  render(<CratesBanner crates={crates("preparing")} where="exercise" />);
  expect(screen.getByRole("status")).toHaveTextContent(
    "外部クレートを準備しています",
  );
  expect(screen.queryByRole("button")).toBeNull();
});

it("failed のときは失敗の案内と再試行のボタンを出し、押すと retry_crate_preparation を呼ぶ", async () => {
  const calls: string[] = [];
  mockIPC((command) => {
    calls.push(command);
    return null;
  });
  render(<CratesBanner crates={crates("failed")} where="home" />);
  const alert = screen.getByRole("alert");
  expect(alert).toHaveTextContent("外部クレートの準備に失敗しました");
  expect(alert).toHaveTextContent("ネットワーク接続を確認してください");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "再試行" }));
  });
  expect(calls).toEqual(["retry_crate_preparation"]);
});

it("再試行を始められなくても、案内は出たままにする", async () => {
  const error = vi.spyOn(console, "error").mockImplementation(() => {});
  mockIPC(() => {
    throw new Error("拒否");
  });
  render(<CratesBanner crates={crates("failed")} where="exercise" />);
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "再試行" }));
  });
  expect(screen.getByRole("alert")).toBeInTheDocument();
  expect(error).toHaveBeenCalled();
  error.mockRestore();
});

it("許可クレートを使う演習だけ、準備が ready でない間は始められない", () => {
  for (const status of ["not_started", "preparing", "failed"] as const) {
    expect(cratesReadyFor(env(status), ["rand"])).toBe(false);
    expect(cratesReadyFor(env(status), [])).toBe(true);
  }
  expect(cratesReadyFor(env("ready"), ["rand"])).toBe(true);
  expect(cratesReadyFor(null, ["rand"])).toBe(true);
});
