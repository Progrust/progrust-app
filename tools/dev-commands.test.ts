// 開発で使う npm のコマンド（documents/spec/architecture/development-environment.md「開発で使うコマンド」「lint と整形」）を確かめる。
import { spawnSync } from "node:child_process";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { ESLint } from "eslint";
import * as prettier from "prettier";
import stylelint from "stylelint";
import { afterEach, describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));

const scripts: Record<string, string> = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
).scripts;

// `a && b && c` を、実行する順のコマンドに分ける。
function steps(script: string): string[] {
  return script.split("&&").map((step) => step.trim());
}

describe("npm run check", () => {
  it("lint、format:check、test を順に実行する", () => {
    expect(steps(scripts.check)).toEqual([
      "npm run lint",
      "npm run format:check",
      "npm test",
    ]);
  });
});

describe("npm test", () => {
  it("Vitest と src-tauri/ の cargo test の両方を実行する", () => {
    const [vitest, cargo, ...rest] = steps(scripts.test);
    expect(vitest).toBe("vitest run");
    expect(cargo).toMatch(/^cargo test /);
    expect(cargo).toContain("--manifest-path src-tauri/Cargo.toml");
    expect(rest).toEqual([]);
  });
});

describe("Clippy の警告", () => {
  const temps: string[] = [];

  afterEach(() => {
    for (const dir of temps.splice(0)) {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  // package.json と、src-tauri/ に小さな crate を置いた一時ディレクトリで、
  // `npm run lint:rust` を実行する。
  function lintRust(libRs: string) {
    const dir = mkdtempSync(join(tmpdir(), "progrust-clippy-"));
    temps.push(dir);
    cpSync(join(root, "package.json"), join(dir, "package.json"));
    mkdirSync(join(dir, "src-tauri", "src"), { recursive: true });
    writeFileSync(
      join(dir, "src-tauri", "Cargo.toml"),
      '[package]\nname = "clippy-fixture"\nversion = "0.1.0"\nedition = "2024"\n',
    );
    writeFileSync(
      join(dir, "src-tauri", "Cargo.lock"),
      'version = 4\n\n[[package]]\nname = "clippy-fixture"\nversion = "0.1.0"\n',
    );
    writeFileSync(join(dir, "src-tauri", "src", "lib.rs"), libRs);
    return spawnSync("npm", ["run", "lint:rust"], {
      cwd: dir,
      encoding: "utf8",
      env: { ...process.env, CARGO_TARGET_DIR: join(dir, "target") },
    });
  }

  it("npm run lint は、Clippy の lint を実行する", () => {
    expect(steps(scripts.lint)).toContain("npm run lint:rust");
  });

  it("警告があると失敗する", () => {
    const result = lintRust(
      "pub fn has_none(v: &[i32]) -> bool {\n    v.len() == 0\n}\n",
    );
    expect(result.status, result.stderr).not.toBe(0);
    expect(result.stderr).toContain("clippy::len_zero");
  }, 120_000);

  it("警告が無ければ通る", () => {
    const result = lintRust(
      "pub fn has_none(v: &[i32]) -> bool {\n    v.is_empty()\n}\n",
    );
    expect(result.status, result.stderr).toBe(0);
  }, 120_000);
});

describe("design-system/ は lint と整形の対象にならない", () => {
  function filesUnder(dir: string): string[] {
    return readdirSync(join(root, dir), {
      recursive: true,
      withFileTypes: true,
    })
      .filter((entry) => entry.isFile())
      .map((entry) => relative(root, join(entry.parentPath, entry.name)));
  }

  // いまのファイルと、これから置かれうる TypeScript・CSS のファイル。
  const designSystemFiles = [
    ...filesUnder("design-system"),
    "design-system/progrust/example.ts",
    "design-system/progrust/example.tsx",
    "design-system/progrust/example.css",
  ];
  const appFiles = ["src/App.tsx", "src/App.css"];

  it("いまのファイルに、CSS がある", () => {
    expect(designSystemFiles.filter((f) => f.endsWith(".css"))).not.toEqual([]);
  });

  it("ESLint の対象にならない", async () => {
    const eslint = new ESLint({ cwd: root });
    for (const file of designSystemFiles) {
      expect(await eslint.isPathIgnored(join(root, file)), file).toBe(true);
    }
    expect(await eslint.isPathIgnored(join(root, "src/App.tsx"))).toBe(false);
  });

  it("stylelint の対象にならない", async () => {
    const cssFiles = designSystemFiles.filter((f) => f.endsWith(".css"));
    const { results } = await stylelint.lint({
      cwd: root,
      files: [...cssFiles.filter((f) => !f.includes("example")), "src/App.css"],
      allowEmptyInput: true,
    });
    const linted = results
      .filter((result) => !result.ignored)
      .map((result) => relative(root, result.source ?? ""));
    expect(linted).toEqual(["src/App.css"]);
  });

  it("Prettier の対象にならない", async () => {
    // Prettier の CLI と同じ ignore のファイルを読む。
    const ignorePath = [".gitignore", ".prettierignore"].map((f) =>
      join(root, f),
    );
    for (const file of designSystemFiles) {
      const info = await prettier.getFileInfo(join(root, file), {
        ignorePath,
      });
      expect(info.ignored, file).toBe(true);
    }
    for (const file of appFiles) {
      const info = await prettier.getFileInfo(join(root, file), {
        ignorePath,
      });
      expect(info.ignored, file).toBe(false);
    }
  });
});
