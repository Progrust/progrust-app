import { describe, expect, it } from "vitest";
import {
  addFile,
  canDelete,
  canRename,
  deleteFile,
  renameFile,
  tabOrder,
  validatePath,
} from "./fileOps";

const files = {
  "src/main.rs": "a",
  "src/geo.rs": "b",
  "tests/it.rs": "c",
};

describe("validatePath", () => {
  it("src/ と tests/ の下の .rs を受け付ける", () => {
    expect(validatePath("src/util.rs", files)).toBeNull();
    expect(validatePath("tests/more.rs", files)).toBeNull();
    expect(validatePath("src/a/b.rs", files)).toBeNull();
  });

  it.each([
    "main.rs",
    "lib/a.rs",
    "srcs/a.rs",
    "src/a.txt",
    "src/a",
    "src/",
    "src/.rs",
    "src/../a.rs",
    "src/./a.rs",
    "src//a.rs",
    "/src/a.rs",
    "src\\a.rs",
    "../src/a.rs",
    "",
  ])("%j は受け付けない", (path) => {
    expect(validatePath(path, files)).not.toBeNull();
  });

  it("同じ名前（大文字と小文字の違いを含む）と、ファイルとフォルダの衝突は受け付けない", () => {
    expect(validatePath("src/geo.rs", files)).not.toBeNull();
    expect(validatePath("src/GEO.rs", files)).not.toBeNull();
    expect(validatePath("src/geo.rs/x.rs", files)).not.toBeNull();
    expect(
      validatePath("tests/it.rs", { "tests/it.rs/x.rs": "" }),
    ).not.toBeNull();
  });
});

describe("削除・名前変更の可否", () => {
  it("src/main.rs と readonly_files のファイルは、削除も名前変更もできない", () => {
    const readonly = ["tests/it.rs"];
    for (const path of ["src/main.rs", "tests/it.rs"]) {
      expect(canDelete(path, readonly)).toBe(false);
      expect(canRename(path, readonly)).toBe(false);
    }
    expect(canDelete("src/geo.rs", readonly)).toBe(true);
    expect(canRename("src/geo.rs", readonly)).toBe(true);
  });
});

describe("ファイルの操作", () => {
  it("追加は末尾に空のファイルを足す", () => {
    expect(Object.keys(addFile(files, "src/x.rs")).at(-1)).toBe("src/x.rs");
    expect(addFile(files, "src/x.rs")["src/x.rs"]).toBe("");
  });

  it("削除はそのファイルだけを除く", () => {
    expect(Object.keys(deleteFile(files, "src/geo.rs"))).toEqual([
      "src/main.rs",
      "tests/it.rs",
    ]);
  });

  it("名前変更は中身と位置を保つ", () => {
    const renamed = renameFile(files, "src/geo.rs", "src/shape.rs");
    expect(Object.entries(renamed)).toEqual([
      ["src/main.rs", "a"],
      ["src/shape.rs", "b"],
      ["tests/it.rs", "c"],
    ]);
  });

  it("タブは src/main.rs を先頭に並べる", () => {
    expect(tabOrder({ "src/a.rs": "", "src/main.rs": "" })).toEqual([
      "src/main.rs",
      "src/a.rs",
    ]);
  });
});
