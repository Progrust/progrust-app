// レッスン専用の CSS の変換（documents/spec/content-format/video/structure.md、ADR 0093）。
// Vite のプラグイン（lesson-videos.ts）と、教材チェックツールの描画（#110）で使う。
import postcss, { type AnyNode, type Document, type Rule } from "postcss";

// レッスンの動画を描く要素を選ぶ属性セレクタ。
export function lessonSelector(lessonId: string): string {
  const escaped = lessonId.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
  return `[data-lesson="${escaped}"]`;
}

// 外側のルールか（入れ子のルールと `@keyframes` のキーフレームは除く）。
// 入れ子のルールは外側のルールからの相対なので、外側に付ければ閉じ込められる。
function isOutermostRule(rule: Rule): boolean {
  for (
    let node: AnyNode | Document | undefined = rule.parent;
    node !== undefined;
    node = node.parent
  ) {
    if (node.type === "rule") {
      return false;
    }
    if (node.type === "atrule" && /keyframes$/i.test(node.name)) {
      return false;
    }
  }
  return true;
}

// 1 つのセレクタを閉じ込める。先頭の `:root` は、レッスンの属性セレクタに置き換える。
function scopeSelector(selector: string, scope: string): string {
  const trimmed = selector.trim();
  const root = /^:root(?![\w-])/.exec(trimmed);
  if (root !== null) {
    return scope + trimmed.slice(root[0].length);
  }
  return `${scope} ${trimmed}`;
}

// すべてのセレクタの前に `[data-lesson="{lessonId}"] ` を付け、`:root` を `[data-lesson="{lessonId}"]` に置き換える。
// セレクタを持たない規則（`@font-face`・`@property` など）は変えない。
export function scopeLessonCss(css: string, lessonId: string): string {
  const scope = lessonSelector(lessonId);
  const root = postcss.parse(css);
  root.walkRules((rule) => {
    if (isOutermostRule(rule)) {
      rule.selectors = rule.selectors.map((s) => scopeSelector(s, scope));
    }
  });
  return root.toString();
}
