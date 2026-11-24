// 開発時の起動（npm run dev）で Vite が前もってまとめる依存に、React が 1 つだけ入ることを確かめる（ADR 0199）。
import { mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { optimizeDeps, resolveConfig } from "vite";
import { afterAll, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const cacheDir = mkdtempSync(join(tmpdir(), "progrust-vite-deps-"));

afterAll(() => {
  rmSync(cacheDir, { recursive: true, force: true });
});

it("いまのリポジトリで、まとめた依存に React の本体が 1 つだけ入る", async () => {
  const config = await resolveConfig(
    {
      root,
      configFile: join(root, "vite.config.ts"),
      cacheDir,
      logLevel: "silent",
    },
    "serve",
  );
  await optimizeDeps(config, true);
  const depsDir = join(cacheDir, "deps");
  // React の本体（react/cjs/react.development.js）が定める関数。1 つの塊に 2 つの React が入ることもある
  // （ADR 0232）ので、塊の数ではなく、全部の塊の中の数を数える。
  const reactCopies = readdirSync(depsDir)
    .filter((file) => file.endsWith(".js"))
    .map(
      (file) =>
        readFileSync(join(depsDir, file), "utf8").split(
          "exports.createElement = ",
        ).length - 1,
    )
    .reduce((sum, count) => sum + count, 0);
  expect(reactCopies).toBe(1);
}, 120_000);
