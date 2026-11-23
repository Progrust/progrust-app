// 許可クレートの一覧と Cargo.lock（documents/spec/exercise-runner/crates-and-workspace.md「許可クレート」）を確かめる。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { allcratesManifest } from "./lock.ts";

const runner = fileURLToPath(new URL("../../runner/", import.meta.url));
const allowed = readFileSync(`${runner}allowed-crates.toml`, "utf8");
const lock = readFileSync(`${runner}Cargo.lock`, "utf8");

/** `[dependencies]` の `名前 = "=版"` の行を { 名前: 版 } にする。 */
function pinnedCrates(toml: string): Record<string, string> {
  const crates: Record<string, string> = {};
  for (const match of toml.matchAll(/^([\w-]+)\s*=\s*"=([^"]+)"\s*$/gm)) {
    crates[match[1]] = match[2];
  }
  return crates;
}

/** Cargo.lock の `[[package]]` を { name, version, dependencies } にする。 */
function lockPackages(text: string) {
  return text
    .split("[[package]]")
    .slice(1)
    .map((block) => ({
      name: /^name = "(.+)"$/m.exec(block)?.[1] ?? "",
      version: /^version = "(.+)"$/m.exec(block)?.[1] ?? "",
      source: /^source = /m.test(block),
      dependencies: [
        ...(/^dependencies = \[([^\]]*)\]/m.exec(block)?.[1] ?? "").matchAll(
          /"([^"]+)"/g,
        ),
      ].map((m) => m[1]),
    }));
}

describe("runner/Cargo.lock", () => {
  const packages = lockPackages(lock);
  const crates = pinnedCrates(allowed);

  it("許可クレートの一覧に、クレートがある", () => {
    expect(Object.keys(crates).length).toBeGreaterThan(0);
  });

  it("ルートのパッケージは progrust_allowed_crates で、ほかにルートが無い", () => {
    const roots = packages.filter((p) => !p.source);
    expect(roots.map((p) => p.name)).toEqual(["progrust_allowed_crates"]);
  });

  it("allowed-crates.toml の全クレートを、固定した版で含み、ルートが依存する", () => {
    const root = packages.find((p) => p.name === "progrust_allowed_crates");
    for (const [name, version] of Object.entries(crates)) {
      expect(
        packages.some((p) => p.name === name && p.version === version),
      ).toBe(true);
      expect(
        root?.dependencies.some((d) => d === name || d.startsWith(`${name} `)),
      ).toBe(true);
    }
  });
});

describe("allcratesManifest", () => {
  it("パッケージ名と edition を決め、[dependencies] を一覧のまま写す", () => {
    const manifest = allcratesManifest(allowed);
    expect(manifest).toContain('name = "progrust_allowed_crates"');
    expect(manifest).toContain('edition = "2024"');
    expect(manifest.endsWith(allowed)).toBe(true);
  });
});
