import { expect, it } from "vitest";
import { build } from "vite";

// `npm run build:web` と同じ設定でビルドし（ファイルには書かない）、出力の CSS を調べる。
it("フロントエンドのビルドの CSS に、書体を外部から読み込む @import が残らない", async () => {
  const result = await build({
    configFile: "vite.config.ts",
    logLevel: "silent",
    build: { write: false },
  });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap((item) =>
    "output" in item ? item.output : [],
  );
  const css = outputs
    .filter((chunk) => chunk.fileName.endsWith(".css"))
    .map((chunk) => (chunk.type === "asset" ? String(chunk.source) : ""))
    .join("\n");
  // デザインシステムの CSS が読み込まれている。
  expect(css).toContain(".pg-");
  expect(css).not.toMatch(/@import/);
  expect(css).not.toMatch(/fonts\.googleapis\.com/);
}, 60_000);
