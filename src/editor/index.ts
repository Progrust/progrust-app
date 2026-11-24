// コードエディタ（Monaco Editor）。
import { monaco } from "./monaco";

export {
  CodeEditor,
  type CodeEditorProps,
  type EditorCommand,
} from "./CodeEditor";

// コマンドのキーを組み立てる（例: KeyMod.CtrlCmd | KeyCode.Enter）。
export const { KeyMod, KeyCode } = monaco;
