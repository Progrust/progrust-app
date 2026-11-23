import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import {
  isDesignSystemBundleCss,
  stripDesignSystemFontImport,
  stripFontImport,
} from "./design-system-css.ts";

it("外部の書体の @import の行を取り除く", () => {
  const css =
    '@import url("https://fonts.googleapis.com/css2?family=Murecho:wght@400;500&display=swap");\n\n/* 土台 */\nbody { margin: 0; }\n';
  expect(stripFontImport(css)).toBe("\n/* 土台 */\nbody { margin: 0; }\n");
});

it("外部ではない @import は取り除かない", () => {
  const css = '@import "./local.css";\nbody { margin: 0; }\n';
  expect(stripFontImport(css)).toBe(css);
});

it("2 つのデザインシステムの bundle.css から @import が無くなる", () => {
  for (const name of ["progrust", "progrust-video"]) {
    const css = readFileSync(
      `design-system/${name}/components/bundle.css`,
      "utf8",
    );
    expect(css).toMatch(/@import/);
    const stripped = stripFontImport(css);
    expect(stripped).not.toMatch(/@import/);
    // URL の中に `;` があっても、行の残りが残らない。
    expect(stripped).not.toMatch(/fonts\.googleapis|wght@/);
    expect(stripped.length).toBe(css.length - css.split("\n")[0].length - 1);
  }
});

it("プラグインは bundle.css だけを変える", () => {
  const plugin = stripDesignSystemFontImport();
  const css = '@import url("https://example.com/a.css");\na { color: red; }\n';
  expect(
    isDesignSystemBundleCss("/x/design-system/progrust/components/bundle.css"),
  ).toBe(true);
  expect(
    plugin.transform(
      css,
      "/x/design-system/progrust-video/components/bundle.css?direct",
    ),
  ).toEqual({ code: "a { color: red; }\n", map: null });
  expect(plugin.transform(css, "/x/src/App.css")).toBeNull();
});
