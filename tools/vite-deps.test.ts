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

it("いまのリポジトリで、まとめた依存のうち React の本体を含む塊が 1 つだけである", async () => {
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
  const withReact = readdirSync(depsDir)
    .filter((file) => file.endsWith(".js"))
    .filter((file) =>
      // React の本体（react/cjs/react.development.js）が定める関数。
      readFileSync(join(depsDir, file), "utf8").includes(
        "exports.createElement = ",
      ),
    );
  expect(withReact).toHaveLength(1);
}, 120_000);
