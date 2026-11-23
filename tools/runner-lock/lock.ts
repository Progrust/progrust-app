// 許可クレートの Cargo.lock を作る（documents/spec/exercise-runner/crates-and-workspace.md「許可クレート」）。
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** 初回準備の `allcrates/` の `Cargo.toml`。`[dependencies]` は許可クレートの一覧のまま。 */
export function allcratesManifest(allowedCrates: string): string {
  return [
    "[package]",
    'name = "progrust_allowed_crates"',
    'version = "0.1.0"',
    'edition = "2024"',
    "",
    allowedCrates,
  ].join("\n");
}

/** `runner/allowed-crates.toml` から `runner/Cargo.lock` を作り直す。 */
export function regenerateLock(runnerDir: string): void {
  const allowed = readFileSync(join(runnerDir, "allowed-crates.toml"), "utf8");
  const temp = mkdtempSync(join(tmpdir(), "progrust-allcrates-"));
  try {
    mkdirSync(join(temp, "src"));
    writeFileSync(join(temp, "Cargo.toml"), allcratesManifest(allowed));
    writeFileSync(join(temp, "src", "lib.rs"), "");
    const result = spawnSync("cargo", ["generate-lockfile"], {
      cwd: temp,
      stdio: "inherit",
    });
    if (result.status !== 0) {
      throw new Error("cargo generate-lockfile に失敗した");
    }
    copyFileSync(join(temp, "Cargo.lock"), join(runnerDir, "Cargo.lock"));
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}
