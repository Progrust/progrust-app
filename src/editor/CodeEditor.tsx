// コードエディタの部品（documents/spec/screens/exercise-screen.md、ADR 0004、ADR 0111、ADR 0225）。
// Monaco Editor で Rust のコードを表示・編集する。補完・LSP は使わない。
import Editor, { type OnMount } from "@monaco-editor/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useCurrentTheme } from "../theme";
import {
  buildEditorTheme,
  editorThemeName,
  readMonoFont,
  readThemeValues,
  type EditorThemeMode,
} from "./editorTheme";
import { monaco } from "./monaco";

// エディタに登録するコマンド（演習画面のショートカット）。
// エディタにフォーカスがあるとき、Monaco の既定の割り当てより優先する（ADR 0111）。
export type EditorCommand = {
  // 1 つのエディタの中で重ならない名前。
  id: string;
  // コマンドの名前（Monaco のコマンドの一覧に出る）。
  label: string;
  // キー。monaco の KeyMod と KeyCode を組み合わせた値（例: KeyMod.CtrlCmd | KeyCode.Enter）。
  keybinding: number;
  run: () => void;
};

export type CodeEditorProps = {
  value: string;
  // 学習者が内容を変えたとき。
  onChange?: (value: string) => void;
  readOnly?: boolean;
  // 文字サイズ（px）。設定の editor_font_size。
  fontSize: number;
  commands?: readonly EditorCommand[];
  // 読み上げ用の名前。
  ariaLabel?: string;
};

type StandaloneEditor = monaco.editor.IStandaloneCodeEditor;

const MODES: readonly EditorThemeMode[] = ["light", "dark"];

// 既定から変える設定。
const BASE_OPTIONS: monaco.editor.IStandaloneEditorConstructionOptions = {
  // 行の高さは、デザインシステムの code の書体のスタイルと同じ（文字サイズの 1.7 倍）。
  lineHeight: 1.7,
  padding: { top: 12, bottom: 12 },
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  automaticLayout: true,
  tabSize: 4,
  insertSpaces: true,
  detectIndentation: false,
  // 補完を出さない（ADR 0004）。
  quickSuggestions: false,
  suggestOnTriggerCharacters: false,
  wordBasedSuggestions: "off",
  parameterHints: { enabled: false },
  inlineSuggest: { enabled: false },
  // 日本語の文字や記号を、紛らわしい文字として枠で囲まない。
  unicodeHighlight: { ambiguousCharacters: false },
};

// Monaco のテーマを、いまの CSS 変数の値から作って登録する（テーマは Monaco 全体で共通）。
function defineThemes() {
  for (const mode of MODES) {
    monaco.editor.defineTheme(
      editorThemeName(mode),
      buildEditorTheme(mode, readThemeValues(mode)),
    );
  }
}

export function CodeEditor({
  value,
  onChange,
  readOnly = false,
  fontSize,
  commands,
  ariaLabel,
}: CodeEditorProps) {
  const mode = useCurrentTheme();
  const [editor, setEditor] = useState<StandaloneEditor | null>(null);
  const [fontFamily] = useState(readMonoFont);

  const options = useMemo(
    (): monaco.editor.IStandaloneEditorConstructionOptions => ({
      ...BASE_OPTIONS,
      readOnly,
      fontSize,
      ...(fontFamily !== null && { fontFamily }),
      ...(ariaLabel !== undefined && { ariaLabel }),
    }),
    [readOnly, fontSize, fontFamily, ariaLabel],
  );

  // 登録したコマンドからは、最新の run を呼ぶ。
  const commandsRef = useRef(commands);
  useEffect(() => {
    commandsRef.current = commands;
  });

  // id とキーが変わったときだけ、登録し直す。
  const commandKeys = (commands ?? [])
    .map((command) => `${command.id}\u0000${command.keybinding}`)
    .join("\u0001");
  useEffect(() => {
    if (editor === null) {
      return;
    }
    const registered = (commandsRef.current ?? []).map((command) =>
      // addAction は、このエディタにだけ効き、破棄できる（addCommand はどちらもできない）。
      editor.addAction({
        id: command.id,
        label: command.label,
        keybindings: [command.keybinding],
        run: () => {
          commandsRef.current?.find((c) => c.id === command.id)?.run();
        },
      }),
    );
    return () => {
      for (const disposable of registered) {
        disposable.dispose();
      }
    };
  }, [editor, commandKeys]);

  const handleMount: OnMount = (mounted) => {
    setEditor(mounted);
    // 書体のファイルを読み終えたら、文字の幅を測り直す（測り直さないと、カーソルの位置がずれる）。
    void document.fonts?.ready.then(() => monaco.editor.remeasureFonts());
  };

  return (
    <Editor
      language="rust"
      value={value}
      onChange={(next) => onChange?.(next ?? "")}
      theme={editorThemeName(mode)}
      options={options}
      beforeMount={defineThemes}
      onMount={handleMount}
      loading={null}
      height="100%"
    />
  );
}
