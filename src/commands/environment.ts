// 環境のコマンド（documents/spec/architecture/commands/environment.md）。
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// cargo・rustc の検出結果。
export type CargoStatus = {
  // found: 必要条件を満たす cargo・rustc が見つかった。version_too_old: 見つかったが rustc が古い。
  status: "found" | "not_found" | "version_too_old";
  // 見つかった cargo・rustc の絶対パス。見つからないときは null。
  cargo_path: string | null;
  rustc_path: string | null;
  // rustc --version から取り出したバージョン（例: 1.100.0）。見つからないときは null。
  rustc_version: string | null;
  // 必要なバージョン。
  required_rustc_version: string;
};

// claude の検出結果。
export type ClaudeStatus = {
  status: "found" | "not_found";
  path: string | null;
  version: string | null;
};

// 初回準備の状態（documents/spec/exercise-runner/crates-and-workspace.md「初回準備の状態」）。
export type CratesStatus = {
  status: "not_started" | "preparing" | "ready" | "failed";
  // ready のときの、初回準備が終わった日時（時差を付けた ISO 8601）。ほかの状態では null。
  prepared_at: string | null;
  // failed のときのエラーの内容。ほかの状態では null。
  error: string | null;
};

// 環境の状態。get_environment_status の戻り値と、イベント environment_updated の内容。
export type EnvironmentStatus = {
  cargo: CargoStatus;
  claude: ClaudeStatus;
  crates: CratesStatus;
};

// 環境の状態を、バックエンドから受け取る。起動時の検出が終わっていなければ、終わってから返る。
export function getEnvironmentStatus(): Promise<EnvironmentStatus> {
  return invoke<EnvironmentStatus>("get_environment_status");
}

// 初回準備に失敗したときに、初回準備をやり直す。始めたら null で解決し、経過と結果は environment_updated で届く。
// 初回準備の状態が failed でないときは invalid_state、cargo が見つからないときは cargo_unavailable で拒否される。
export function retryCratePreparation(): Promise<null> {
  return invoke<null>("retry_crate_preparation");
}

// 環境の状態が変わったときのイベント。内容は、変わった後の環境の状態（全体）。
const ENVIRONMENT_UPDATED = "environment_updated";

// environment_updated の受け取りを始める。受け取りをやめる関数で解決する。
// やめるのに失敗しても（画面を離れる途中など）、何もしない。
export async function listenEnvironmentUpdated(
  handler: (status: EnvironmentStatus) => void,
): Promise<() => void> {
  const unlisten = await listen<EnvironmentStatus>(
    ENVIRONMENT_UPDATED,
    (event) => handler(event.payload),
  );
  return () => {
    // listen の返す関数は、実際には Promise を返す。
    Promise.resolve(unlisten()).catch(() => {});
  };
}
