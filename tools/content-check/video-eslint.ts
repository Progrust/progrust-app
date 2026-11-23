// 動画のコード（`.tsx`・`.ts`）の禁止事項を検査する ESLint の設定と独自の規則
// （documents/spec/content-format/video/prohibited-code.md）。
// アプリの lint（eslint.config.ts）とは別の設定で、禁止事項だけを検査する。
import { builtinModules } from "node:module";
import type { Linter, Rule } from "eslint";
import globals from "globals";
import tseslint from "typescript-eslint";

// ---- 色の直接指定 ----

// 16 進表記。`&#123;`（文字参照）は除く。
const HEX =
  /(?<!&)#(?:[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{4}|[0-9a-f]{3})(?![0-9a-z_-])/i;
// 色の関数。
const COLOR_FUNCTION =
  /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color|color-mix|device-cmyk|light-dark)\s*\(/i;
// SVG の参照（`url(#grad)`）は色ではないので、16 進表記の検査の前に除く。
const URL_FUNCTION = /url\([^)]*\)/gi;

// CSS の色の名前（CSS Color Module Level 4 の named colors）。
// `none`・`currentColor`・`transparent`・`inherit` は書いてよいので含めない。
export const NAMED_COLORS = new Set([
  "aliceblue",
  "antiquewhite",
  "aqua",
  "aquamarine",
  "azure",
  "beige",
  "bisque",
  "black",
  "blanchedalmond",
  "blue",
  "blueviolet",
  "brown",
  "burlywood",
  "cadetblue",
  "chartreuse",
  "chocolate",
  "coral",
  "cornflowerblue",
  "cornsilk",
  "crimson",
  "cyan",
  "darkblue",
  "darkcyan",
  "darkgoldenrod",
  "darkgray",
  "darkgreen",
  "darkgrey",
  "darkkhaki",
  "darkmagenta",
  "darkolivegreen",
  "darkorange",
  "darkorchid",
  "darkred",
  "darksalmon",
  "darkseagreen",
  "darkslateblue",
  "darkslategray",
  "darkslategrey",
  "darkturquoise",
  "darkviolet",
  "deeppink",
  "deepskyblue",
  "dimgray",
  "dimgrey",
  "dodgerblue",
  "firebrick",
  "floralwhite",
  "forestgreen",
  "fuchsia",
  "gainsboro",
  "ghostwhite",
  "gold",
  "goldenrod",
  "gray",
  "green",
  "greenyellow",
  "grey",
  "honeydew",
  "hotpink",
  "indianred",
  "indigo",
  "ivory",
  "khaki",
  "lavender",
  "lavenderblush",
  "lawngreen",
  "lemonchiffon",
  "lightblue",
  "lightcoral",
  "lightcyan",
  "lightgoldenrodyellow",
  "lightgray",
  "lightgreen",
  "lightgrey",
  "lightpink",
  "lightsalmon",
  "lightseagreen",
  "lightskyblue",
  "lightslategray",
  "lightslategrey",
  "lightsteelblue",
  "lightyellow",
  "lime",
  "limegreen",
  "linen",
  "magenta",
  "maroon",
  "mediumaquamarine",
  "mediumblue",
  "mediumorchid",
  "mediumpurple",
  "mediumseagreen",
  "mediumslateblue",
  "mediumspringgreen",
  "mediumturquoise",
  "mediumvioletred",
  "midnightblue",
  "mintcream",
  "mistyrose",
  "moccasin",
  "navajowhite",
  "navy",
  "oldlace",
  "olive",
  "olivedrab",
  "orange",
  "orangered",
  "orchid",
  "palegoldenrod",
  "palegreen",
  "paleturquoise",
  "palevioletred",
  "papayawhip",
  "peachpuff",
  "peru",
  "pink",
  "plum",
  "powderblue",
  "purple",
  "rebeccapurple",
  "red",
  "rosybrown",
  "royalblue",
  "saddlebrown",
  "salmon",
  "sandybrown",
  "seagreen",
  "seashell",
  "sienna",
  "silver",
  "skyblue",
  "slateblue",
  "slategray",
  "slategrey",
  "snow",
  "springgreen",
  "steelblue",
  "tan",
  "teal",
  "thistle",
  "tomato",
  "turquoise",
  "violet",
  "wheat",
  "white",
  "whitesmoke",
  "yellow",
  "yellowgreen",
]);

// 色を値にとる style のプロパティ（`-` を除き、小文字にした名前）。
// 名前が `color` で終わるもの（`backgroundColor`・`stopColor` など）も含める。
const COLOR_STYLE_KEYS = new Set([
  "background",
  "backgroundimage",
  "border",
  "bordertop",
  "borderright",
  "borderbottom",
  "borderleft",
  "borderblock",
  "borderblockstart",
  "borderblockend",
  "borderinline",
  "borderinlinestart",
  "borderinlineend",
  "outline",
  "boxshadow",
  "textshadow",
  "textdecoration",
  "textemphasis",
  "columnrule",
  "fill",
  "stroke",
  "filter",
  "webkittextstroke",
]);

function isColorStyleKey(key: string): boolean {
  const k = key.replace(/-/g, "").toLowerCase();
  return k.endsWith("color") || COLOR_STYLE_KEYS.has(k);
}

// 色を値にとる SVG の属性。
const COLOR_ATTRIBUTES = new Set([
  "fill",
  "stroke",
  "color",
  "stopColor",
  "stop-color",
  "floodColor",
  "flood-color",
  "lightingColor",
  "lighting-color",
]);

// 値の文字列に含まれる色の名前。CSS 変数の名前（`--red`）、`url(…)`、引用符の中は除く。
export function namedColorsIn(text: string): string[] {
  const cleaned = text
    .toLowerCase()
    .replace(URL_FUNCTION, " ")
    .replace(/--[\w-]+/g, " ")
    .replace(/'[^']*'|"[^"]*"/g, " ");
  return (cleaned.match(/[a-z-]+/g) ?? []).filter((w) => NAMED_COLORS.has(w));
}

// ESLint の規則が受け取る節。JSX の節は estree の型に無いので、ここで最小限の形を決める。
interface AstNode {
  type: string;
  parent?: AstNode;
  [key: string]: unknown;
}

function asNode(node: unknown): AstNode {
  return node as AstNode;
}

function child(node: AstNode, key: string): AstNode | undefined {
  const value = node[key];
  return typeof value === "object" && value !== null
    ? (value as AstNode)
    : undefined;
}

// 値に直接書かれた文字列（文字列リテラル、テンプレートの文字列の部分、条件の両側）。
function directStrings(node: AstNode | undefined): {
  node: AstNode;
  text: string;
}[] {
  if (!node) return [];
  switch (node.type) {
    case "Literal":
      return typeof node.value === "string" ? [{ node, text: node.value }] : [];
    case "TemplateLiteral":
      return ((node.quasis as AstNode[]) ?? []).map((q) => ({
        node: q,
        text: templateText(q),
      }));
    case "ConditionalExpression":
      return [
        ...directStrings(child(node, "consequent")),
        ...directStrings(child(node, "alternate")),
      ];
    case "LogicalExpression":
      return [
        ...directStrings(child(node, "left")),
        ...directStrings(child(node, "right")),
      ];
    case "TSAsExpression":
    case "TSSatisfiesExpression":
    case "JSXExpressionContainer":
      return directStrings(child(node, "expression"));
    default:
      return [];
  }
}

function templateText(node: AstNode): string {
  const value = node.value as { cooked?: string | null; raw: string };
  return value.cooked ?? value.raw;
}

function propertyKey(node: AstNode): string | undefined {
  if (node.computed) return undefined;
  const key = child(node, "key");
  if (!key) return undefined;
  if (key.type === "Identifier") return key.name as string;
  if (key.type === "Literal" && typeof key.value === "string") return key.value;
  return undefined;
}

// JSX の `style={{ … }}` に直接書いたオブジェクトか。
function isStyleObject(node: AstNode | undefined): boolean {
  const container = node?.parent;
  const attribute = container?.parent;
  return (
    container?.type === "JSXExpressionContainer" &&
    attribute?.type === "JSXAttribute" &&
    child(attribute, "name")?.name === "style"
  );
}

const noHardcodedColor: Rule.RuleModule = {
  meta: {
    type: "problem",
    schema: [],
    messages: {
      hex: "色を 16 進表記（{{value}}）で直接書いている。動画のデザインシステムのトークン（CSS 変数）を使う",
      func: "色を色の関数（{{value}}）で直接書いている。動画のデザインシステムのトークン（CSS 変数）を使う",
      named:
        "色を色の名前（{{value}}）で直接書いている。動画のデザインシステムのトークン（CSS 変数）を使う",
    },
  },
  create(context) {
    // 16 進表記と色の関数は、どの文字列でも検査する。
    const checkAnyString = (node: AstNode, text: string) => {
      const stripped = text.replace(URL_FUNCTION, " ");
      const hex = stripped.match(HEX);
      if (hex) {
        context.report({
          node: node as unknown as Rule.Node,
          messageId: "hex",
          data: { value: hex[0] },
        });
        return;
      }
      const func = text.match(COLOR_FUNCTION);
      if (func) {
        context.report({
          node: node as unknown as Rule.Node,
          messageId: "func",
          data: { value: func[0].replace(/\s*\($/, "()") },
        });
      }
    };
    // 色の名前は、style のプロパティの値と SVG の色の属性に直接書いたものだけを検査する。
    const checkNamed = (value: AstNode | undefined) => {
      for (const s of directStrings(value)) {
        const names = namedColorsIn(s.text);
        if (names.length > 0) {
          context.report({
            node: s.node as unknown as Rule.Node,
            messageId: "named",
            data: { value: names.join("、") },
          });
        }
      }
    };
    return {
      Literal(node) {
        const n = asNode(node);
        if (typeof n.value === "string") checkAnyString(n, n.value);
      },
      TemplateElement(node) {
        const n = asNode(node);
        checkAnyString(n, templateText(n));
      },
      JSXText(node: Rule.Node) {
        const n = asNode(node);
        checkAnyString(n, n.value as string);
      },
      Property(node) {
        const n = asNode(node);
        const key = propertyKey(n);
        if (key === undefined) return;
        if (isColorStyleKey(key) || isStyleObject(n.parent)) {
          checkNamed(child(n, "value"));
        }
      },
      JSXAttribute(node: Rule.Node) {
        const n = asNode(node);
        const name = child(n, "name");
        if (
          name?.type === "JSXIdentifier" &&
          COLOR_ATTRIBUTES.has(name.name as string)
        ) {
          checkNamed(child(n, "value"));
        }
      },
    };
  },
};

// ---- 外部の URL と file: の URL ----

const EXTERNAL_URL = /^\s*(?:https?:|file:|\/\/[^\s/])/i;
const EXTERNAL_CSS_URL = /url\(\s*['"]?\s*(?:https?:|file:|\/\/[^\s/])/i;

// import などのモジュールの指定は、no-restricted-module が検査する。
function isModuleSource(node: AstNode): boolean {
  const parent = node.parent;
  if (!parent) return false;
  return (
    ((parent.type === "ImportDeclaration" ||
      parent.type === "ExportNamedDeclaration" ||
      parent.type === "ExportAllDeclaration" ||
      parent.type === "ImportExpression") &&
      parent.source === node) ||
    parent.type === "TSExternalModuleReference"
  );
}

const noExternalUrl: Rule.RuleModule = {
  meta: {
    type: "problem",
    schema: [],
    messages: {
      url: "外部の URL・file: の URL（{{value}}）を参照している",
    },
  },
  create(context) {
    const check = (node: AstNode, text: string) => {
      const match = EXTERNAL_URL.test(text) || EXTERNAL_CSS_URL.test(text);
      if (match) {
        context.report({
          node: node as unknown as Rule.Node,
          messageId: "url",
          data: { value: text.trim().slice(0, 60) },
        });
      }
    };
    return {
      Literal(node) {
        const n = asNode(node);
        if (typeof n.value === "string" && !isModuleSource(n)) {
          check(n, n.value);
        }
      },
      TemplateElement(node) {
        const n = asNode(node);
        check(n, templateText(n));
      },
    };
  },
};

// ---- import するモジュール ----

const BUILTIN_MODULES = new Set(builtinModules);

// 禁止するモジュールなら、その理由を返す。
export function restrictedModuleReason(specifier: string): string | undefined {
  if (specifier.startsWith("@tauri-apps/")) {
    return "Tauri の API（@tauri-apps/）は使えない";
  }
  if (/^(?:https?:|\/\/)/i.test(specifier)) {
    return "外部の URL は参照できない";
  }
  if (/^file:/i.test(specifier)) return "file: の URL は参照できない";
  if (
    specifier.startsWith("node:") ||
    BUILTIN_MODULES.has(specifier) ||
    BUILTIN_MODULES.has(specifier.split("/")[0])
  ) {
    return "Node.js の組み込みモジュールは使えない";
  }
  return undefined;
}

const noRestrictedModule: Rule.RuleModule = {
  meta: {
    type: "problem",
    schema: [],
    messages: {
      module: "{{value}} を import している。{{reason}}",
      dynamic:
        "動的 import の引数は、文字列リテラルで書く（検査をすり抜ける書き方は使えない）",
    },
  },
  create(context) {
    const check = (source: AstNode | undefined) => {
      if (
        !source ||
        source.type !== "Literal" ||
        typeof source.value !== "string"
      ) {
        return false;
      }
      const reason = restrictedModuleReason(source.value);
      if (reason) {
        context.report({
          node: source as unknown as Rule.Node,
          messageId: "module",
          data: { value: source.value, reason },
        });
      }
      return true;
    };
    const fromSource = (node: unknown) => {
      check(child(asNode(node), "source"));
    };
    return {
      ImportDeclaration: fromSource,
      ExportNamedDeclaration: fromSource,
      ExportAllDeclaration: fromSource,
      ImportExpression(node) {
        const n = asNode(node);
        if (!check(child(n, "source"))) {
          context.report({ node, messageId: "dynamic" });
        }
      },
      TSExternalModuleReference(node: Rule.Node) {
        check(child(asNode(node), "expression"));
      },
    };
  },
};

// ---- 標準の規則の指定 ----

// window などの、ブラウザの全体を指すオブジェクト。
const GLOBAL_OBJECTS = "window|globalThis|self|navigator";
// ブラウザの全体を指すオブジェクトのうち、そのプロパティから fetch などを取り出せるもの。
const BROWSER_OBJECTS = "window|globalThis|self|top|parent|frames|navigator";
const TAURI_NAMES = "__TAURI__|__TAURI_INTERNALS__";
const NETWORK_NAMES = "fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon";
const EVAL_NAMES = "eval|Function";

// window.fetch・window.navigator.sendBeacon のような、ブラウザの全体を指すオブジェクトのプロパティ。
function browserMember(names: string): string {
  const member = `MemberExpression[computed=false][property.name=/^(?:${names})$/]`;
  return [
    `${member}[object.type='Identifier'][object.name=/^(?:${BROWSER_OBJECTS})$/]`,
    `${member}[object.type='MemberExpression'][object.property.name=/^(?:${BROWSER_OBJECTS})$/]`,
  ].join(", ");
}

// const { fetch } = window のような、ブラウザの全体を指すオブジェクトからの分割代入。
function browserDestructuring(names: string): string {
  const property = `ObjectPattern > Property[key.name=/^(?:${names})$/]`;
  return [
    `VariableDeclarator[init.type='Identifier'][init.name=/^(?:${BROWSER_OBJECTS})$/] > ${property}`,
    `AssignmentExpression[right.type='Identifier'][right.name=/^(?:${BROWSER_OBJECTS})$/] > ${property}`,
  ].join(", ");
}

const restrictedSyntax: { selector: string; message: string }[] = [
  {
    selector: `Identifier[name=/^(?:${TAURI_NAMES})$/], Literal[value=/^(?:${TAURI_NAMES})$/]`,
    message: "Tauri の API（__TAURI__・__TAURI_INTERNALS__）は使えない",
  },
  {
    selector: `${browserMember(NETWORK_NAMES)}, ${browserDestructuring(NETWORK_NAMES)}`,
    message:
      "ネットワークへのアクセス（fetch・XMLHttpRequest・WebSocket・EventSource・navigator.sendBeacon）は使えない",
  },
  {
    selector: "CallExpression[callee.type='Identifier'][callee.name='require']",
    message: "require() は使えない（ファイルへのアクセス）",
  },
  {
    selector: `NewExpression[callee.name='Function'], CallExpression[callee.name='Function'], ${browserMember(EVAL_NAMES)}, ${browserDestructuring(EVAL_NAMES)}`,
    message: "eval・new Function は使えない（検査をすり抜ける書き方）",
  },
  {
    selector: `MemberExpression[computed=true][object.type='Identifier'][object.name=/^(?:${GLOBAL_OBJECTS})$/]`,
    message:
      "window・globalThis・self・navigator への [...] でのアクセスは使えない（検査をすり抜ける書き方）",
  },
  {
    selector: `TSAsExpression > Identifier.expression[name=/^(?:${GLOBAL_OBJECTS})$/], TSTypeAssertion > Identifier.expression[name=/^(?:${GLOBAL_OBJECTS})$/]`,
    message:
      "window・globalThis・self・navigator を型アサーションして使えない（検査をすり抜ける書き方）",
  },
  {
    selector: "Identifier[name='staticFile']",
    message:
      "Remotion の staticFile() は使えない。画像は assets/ に置いて import する",
  },
];

export const VIDEO_ESLINT_PLUGIN = "progrust-video";

// 動画のコードの ESLint の設定。インラインの指定（eslint-disable など）は効かない。
export const videoEslintConfig: Linter.Config[] = [
  {
    files: ["**/*.ts", "**/*.tsx"],
    languageOptions: {
      parser: tseslint.parser as Linter.Parser,
      ecmaVersion: "latest",
      sourceType: "module",
      globals: globals.browser,
    },
    linterOptions: {
      noInlineConfig: true,
      reportUnusedDisableDirectives: "off",
    },
    plugins: {
      [VIDEO_ESLINT_PLUGIN]: {
        rules: {
          "no-hardcoded-color": noHardcodedColor,
          "no-external-url": noExternalUrl,
          "no-restricted-module": noRestrictedModule,
        },
      },
    },
    rules: {
      [`${VIDEO_ESLINT_PLUGIN}/no-hardcoded-color`]: "error",
      [`${VIDEO_ESLINT_PLUGIN}/no-external-url`]: "error",
      [`${VIDEO_ESLINT_PLUGIN}/no-restricted-module`]: "error",
      "no-restricted-globals": [
        "error",
        ...["fetch", "XMLHttpRequest", "WebSocket", "EventSource"].map(
          (name) => ({
            name,
            message: "ネットワークへのアクセスは使えない",
          }),
        ),
        {
          name: "eval",
          message: "eval は使えない（検査をすり抜ける書き方）",
        },
      ],
      "no-restricted-syntax": ["error", ...restrictedSyntax],
    },
  },
];
