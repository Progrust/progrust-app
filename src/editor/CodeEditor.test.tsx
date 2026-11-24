import { act, render } from "@testing-library/react";
import type { EditorProps } from "@monaco-editor/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { CodeEditor, type EditorCommand } from "./CodeEditor";

// jsdom では Monaco を描画できないので、Monaco と @monaco-editor/react を偽物に置き換え、
// 部品が渡すもの（設定・テーマ・コマンド）を確かめる。
const fake = vi.hoisted(() => {
  type Action = {
    id: string;
    label: string;
    keybindings: number[];
    run: () => void;
    disposed: boolean;
  };
  const state = {
    props: null as EditorProps | null,
    actions: [] as Action[],
    defineTheme: (() => {}) as (name: string, data: unknown) => void,
  };
  const editor = {
    addAction(descriptor: Omit<Action, "disposed">) {
      const action = { ...descriptor, disposed: false };
      state.actions.push(action);
      return {
        dispose: () => {
          action.disposed = true;
        },
      };
    },
  };
  return { state, editor };
});

vi.mock("./monaco", () => ({
  monaco: {
    editor: {
      defineTheme: (name: string, data: unknown) =>
        fake.state.defineTheme(name, data),
      remeasureFonts: () => {},
    },
  },
}));

vi.mock("@monaco-editor/react", async () => {
  const { useEffect, useRef } = await import("react");
  function Editor(props: EditorProps) {
    fake.state.props = props;
    const mounted = useRef(false);
    useEffect(() => {
      // 一度だけ作る（実物と同じ）。
      if (!mounted.current) {
        mounted.current = true;
        props.beforeMount?.({} as never);
        props.onMount?.(fake.editor as never, {} as never);
      }
    });
    return <div data-testid="editor" />;
  }
  return { default: Editor };
});

const live = () => fake.state.actions.filter((action) => !action.disposed);

beforeEach(() => {
  fake.state.props = null;
  fake.state.actions = [];
  fake.state.defineTheme = vi.fn();
  document.documentElement.dataset.theme = "light";
});

afterEach(() => {
  delete document.documentElement.dataset.theme;
});

it("内容・言語・文字サイズ・読み取り専用を Monaco に渡し、補完を出さない", () => {
  render(<CodeEditor value="fn main() {}" fontSize={18} readOnly />);
  const props = fake.state.props!;
  expect(props.value).toBe("fn main() {}");
  expect(props.language).toBe("rust");
  expect(props.options).toMatchObject({
    fontSize: 18,
    readOnly: true,
    quickSuggestions: false,
    suggestOnTriggerCharacters: false,
    wordBasedSuggestions: "off",
  });
});

it("読み取り専用を指定しなければ、編集できる", () => {
  render(<CodeEditor value="" fontSize={14} />);
  expect(fake.state.props!.options?.readOnly).toBe(false);
});

it("文字サイズを変えると、Monaco に新しい文字サイズを渡す", () => {
  const { rerender } = render(<CodeEditor value="" fontSize={14} />);
  rerender(<CodeEditor value="" fontSize={20} />);
  expect(fake.state.props!.options?.fontSize).toBe(20);
});

it("変更を、内容の文字列で知らせる", () => {
  const onChange = vi.fn();
  render(<CodeEditor value="" fontSize={14} onChange={onChange} />);
  fake.state.props!.onChange?.("let x = 1;", {} as never);
  fake.state.props!.onChange?.(undefined, {} as never);
  expect(onChange.mock.calls).toEqual([["let x = 1;"], [""]]);
});

it("ライト・ダークのテーマを登録し、アプリのテーマに合わせて切り替える", async () => {
  render(<CodeEditor value="" fontSize={14} />);
  expect(
    vi.mocked(fake.state.defineTheme).mock.calls.map(([name]) => name),
  ).toEqual(["progrust-light", "progrust-dark"]);
  expect(fake.state.props!.theme).toBe("progrust-light");

  await act(async () => {
    document.documentElement.dataset.theme = "dark";
  });
  expect(fake.state.props!.theme).toBe("progrust-dark");
});

it("コマンドを、そのエディタのアクションとしてキーとともに登録し、最新の処理を呼ぶ", () => {
  const first = vi.fn();
  const latest = vi.fn();
  const command = (run: () => void): EditorCommand[] => [
    { id: "run", label: "実行", keybinding: 2051, run },
  ];
  const { rerender } = render(
    <CodeEditor value="" fontSize={14} commands={command(first)} />,
  );
  expect(live()).toHaveLength(1);
  expect(live()[0]).toMatchObject({
    id: "run",
    label: "実行",
    keybindings: [2051],
  });

  // 処理だけが変わったときは登録し直さず、最新の処理を呼ぶ。
  rerender(<CodeEditor value="" fontSize={14} commands={command(latest)} />);
  expect(fake.state.actions).toHaveLength(1);
  live()[0].run();
  expect(first).not.toHaveBeenCalled();
  expect(latest).toHaveBeenCalledTimes(1);
});

it("キーが変わったら登録し直し、外したら破棄する", () => {
  const run = vi.fn();
  const { rerender, unmount } = render(
    <CodeEditor
      value=""
      fontSize={14}
      commands={[{ id: "run", label: "実行", keybinding: 1, run }]}
    />,
  );
  rerender(
    <CodeEditor
      value=""
      fontSize={14}
      commands={[{ id: "run", label: "実行", keybinding: 2, run }]}
    />,
  );
  expect(live().map((action) => action.keybindings)).toEqual([[2]]);

  unmount();
  expect(live()).toHaveLength(0);
});
