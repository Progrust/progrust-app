// Monaco Editor の入口（documents/spec/architecture/tech-stack.md、ADR 0111、ADR 0225）。
// CDN を読まず、アプリのビルドに同梱した monaco-editor を使う。Web Worker も同梱する。
// 言語は Rust だけを登録し、補完・LSP は使わない（ADR 0004）。
//
// monaco-editor の既定の入口（esm/vs/editor/editor.main.js）は、全言語の定義と、
// css・html・json・typescript の言語機能と、LSP のクライアントを読み込む。ここでは、それらを除き、
// 編集機能（contrib）を既定の入口と同じ順に読み込む。版を上げて既定の入口の一覧が変わると、
// monaco.test.ts が失敗する。そのときは、下の一覧を既定の入口に合わせて直す。
//
// import の指定子に esm/vs/ を付けない（package.json の exports が付ける）。
// .css は exports に当たらないので、相対パスで読む
// （documents/verification/issue-21-monaco-in-tauri.md）。
//
// エディタの中の文言（右クリックのメニュー、検索の欄など）は、同梱の日本語の翻訳で日本語にする（ADR 0230）。
// 翻訳は文言の一覧をグローバルな値に入れるだけなので、他の Monaco のモジュールより先に読み込む。
// 必ず最初の import にする（monaco.test.ts で確かめる）。
import "monaco-editor/nls/lang/ja";
import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor/editor/editor.api";
import EditorWorker from "monaco-editor/editor/editor.worker?worker";
import {
  conf as rustConf,
  language as rustBaseLanguage,
} from "monaco-editor/languages/definitions/rust/rust";
import { extendRustLanguage } from "./rustLanguage";

// ---- 編集機能（editor.main.js の一覧から、言語と LSP を除いたもの。順も同じ） ----
import "monaco-editor/editor/contrib/anchorSelect/browser/anchorSelect";
import "monaco-editor/editor/contrib/bracketMatching/browser/bracketMatching";
import "monaco-editor/editor/contrib/caretOperations/browser/transpose";
import "monaco-editor/editor/contrib/clipboard/browser/clipboard";
import "monaco-editor/editor/contrib/codeAction/browser/codeActionContributions";
import "monaco-editor/editor/browser/widget/codeEditor/codeEditorWidget";
import "monaco-editor/editor/contrib/codelens/browser/codelensController";
import "../../node_modules/monaco-editor/esm/vs/base/browser/ui/codicons/codicon/codicon.css";
import "monaco-editor/editor/contrib/colorPicker/browser/colorPickerContribution";
import "monaco-editor/editor/contrib/comment/browser/comment";
import "monaco-editor/editor/contrib/contextmenu/browser/contextmenu";
import "monaco-editor/editor/contrib/cursorUndo/browser/cursorUndo";
import "monaco-editor/editor/browser/widget/diffEditor/diffEditor.contribution";
import "monaco-editor/editor/contrib/diffEditorBreadcrumbs/browser/contribution";
import "monaco-editor/editor/contrib/dnd/browser/dnd";
import "monaco-editor/editor/contrib/documentSymbols/browser/documentSymbols";
import "monaco-editor/editor/contrib/dropOrPasteInto/browser/dropIntoEditorContribution";
import "monaco-editor/features/find/register";
import "monaco-editor/editor/contrib/floatingMenu/browser/floatingMenu.contribution";
import "monaco-editor/editor/contrib/folding/browser/folding";
import "monaco-editor/editor/contrib/fontZoom/browser/fontZoom";
import "monaco-editor/editor/contrib/format/browser/formatActions";
import "monaco-editor/editor/contrib/gotoError/browser/gotoError";
import "monaco-editor/editor/standalone/browser/quickAccess/standaloneGotoLineQuickAccess";
import "monaco-editor/editor/contrib/gotoSymbol/browser/link/goToDefinitionAtPosition";
import "monaco-editor/editor/contrib/gpu/browser/gpuActions";
import "monaco-editor/editor/contrib/hover/browser/hoverContribution";
import "monaco-editor/editor/contrib/indentation/browser/indentation";
import "monaco-editor/editor/contrib/inlayHints/browser/inlayHintsContribution";
import "monaco-editor/editor/contrib/inlineCompletions/browser/inlineCompletions.contribution";
import "monaco-editor/editor/contrib/inlineProgress/browser/inlineProgress";
import "monaco-editor/editor/contrib/inPlaceReplace/browser/inPlaceReplace";
import "monaco-editor/editor/contrib/insertFinalNewLine/browser/insertFinalNewLine";
import "monaco-editor/editor/standalone/browser/inspectTokens/inspectTokens";
import "monaco-editor/editor/standalone/browser/iPadShowKeyboard/iPadShowKeyboard";
import "monaco-editor/editor/contrib/lineSelection/browser/lineSelection";
import "monaco-editor/editor/contrib/linesOperations/browser/linesOperations";
import "monaco-editor/editor/contrib/linkedEditing/browser/linkedEditing";
import "monaco-editor/editor/contrib/links/browser/links";
import "monaco-editor/editor/contrib/longLinesHelper/browser/longLinesHelper";
import "monaco-editor/editor/contrib/middleScroll/browser/middleScroll.contribution";
import "monaco-editor/editor/contrib/multicursor/browser/multicursor";
import "monaco-editor/editor/contrib/parameterHints/browser/parameterHints";
import "monaco-editor/editor/contrib/placeholderText/browser/placeholderText.contribution";
import "monaco-editor/editor/standalone/browser/quickAccess/standaloneCommandsQuickAccess";
import "monaco-editor/editor/standalone/browser/quickAccess/standaloneHelpQuickAccess";
import "monaco-editor/editor/standalone/browser/quickAccess/standaloneGotoSymbolQuickAccess";
import "monaco-editor/editor/contrib/readOnlyMessage/browser/contribution";
import "monaco-editor/editor/standalone/browser/referenceSearch/standaloneReferenceSearch";
import "monaco-editor/editor/contrib/rename/browser/rename";
import "monaco-editor/editor/contrib/sectionHeaders/browser/sectionHeaders";
import "monaco-editor/editor/contrib/semanticTokens/browser/viewportSemanticTokens";
import "monaco-editor/editor/contrib/smartSelect/browser/smartSelect";
import "monaco-editor/editor/contrib/snippet/browser/snippetController2";
import "monaco-editor/editor/contrib/stickyScroll/browser/stickyScrollContribution";
import "monaco-editor/editor/contrib/suggest/browser/suggestInlineCompletions";
import "monaco-editor/editor/standalone/browser/toggleHighContrast/toggleHighContrast";
import "monaco-editor/editor/contrib/toggleTabFocusMode/browser/toggleTabFocusMode";
import "monaco-editor/editor/contrib/tokenization/browser/tokenization";
import "monaco-editor/editor/contrib/unicodeHighlighter/browser/unicodeHighlighter";
import "monaco-editor/editor/contrib/unusualLineTerminators/browser/unusualLineTerminators";
import "monaco-editor/editor/contrib/wordHighlighter/browser/wordHighlighter";
import "monaco-editor/editor/contrib/wordOperations/browser/wordOperations";
import "monaco-editor/editor/contrib/wordPartOperations/browser/wordPartOperations";
import "monaco-editor/editor/browser/coreCommands";
import "monaco-editor/editor/contrib/caretOperations/browser/caretOperations";
import "monaco-editor/editor/contrib/dropOrPasteInto/browser/copyPasteContribution";
import "monaco-editor/editor/contrib/find/browser/findController";
import "monaco-editor/editor/contrib/gotoSymbol/browser/goToCommands";
import "monaco-editor/editor/contrib/gotoError/browser/markerSelectionStatus";
import "monaco-editor/editor/contrib/semanticTokens/browser/documentSemanticTokens";
import "monaco-editor/editor/contrib/structuralDiff/common/structuralDiff.contribution";
import "monaco-editor/editor/contrib/suggest/browser/suggestController";
import "monaco-editor/editor/common/standaloneStrings";
import "../../node_modules/monaco-editor/esm/vs/base/browser/ui/codicons/codicon/codicon-modifiers.css";
// ---- 編集機能ここまで ----

// Web Worker は、同梱のファイルから作る（worker-src 'self' で動く）。
self.MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

// Rust を登録する。Monaco の Rust の登録（languages/definitions/rust/register）は、
// 字句の定義を後から遅れて読み込み、ここで設定する定義を上書きするので使わない。
monaco.languages.register({
  id: "rust",
  extensions: [".rs"],
  aliases: ["Rust", "rust"],
});
monaco.languages.setLanguageConfiguration("rust", rustConf);
monaco.languages.setMonarchTokensProvider(
  "rust",
  extendRustLanguage(rustBaseLanguage),
);

// @monaco-editor/react のローダーに同梱の monaco を渡す。渡さないと CDN（jsdelivr）から読む。
loader.config({ monaco });

export { monaco };
