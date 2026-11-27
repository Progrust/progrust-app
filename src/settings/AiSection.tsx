// 設定画面の「AI」の区分。決まりは documents/spec/screens/settings-screen.md「設定項目」、
// documents/spec/claude-integration/model-selection.md、
// documents/spec/claude-integration/unavailable.md。配置は design-system/ui-design/Settings.dc.html。
import {
  ChevronDown,
  CircleAlert,
  CircleCheck,
  LoaderCircle,
} from "lucide-react";
import { useState } from "react";
import type { SubmitEvent } from "react";
import { aiErrorMessage } from "../claude/aiErrorMessage";
import { asCommandError } from "../commands/error";
import { testClaudeConnection } from "../commands/settings";
import type { ConnectionTestResult, Settings } from "../commands/settings";
import { openInstallGuide } from "../environment/installGuide";
import { Row, Section } from "./SettingsParts";

type ModelKey = keyof Settings["models"];

const ALIASES = ["opus", "sonnet", "haiku"];
const OTHER = "other";

const ICON = { size: 14, strokeWidth: 2.4, "aria-hidden": true } as const;

const isAlias = (model: string) => ALIASES.includes(model);

// models: 保存済みのモデル。onSave: 保存して、保存できなかったときは reject する。
export function AiSection({
  models,
  onSave,
}: {
  models: Settings["models"];
  onSave: (key: ModelKey, model: string) => Promise<void>;
}) {
  return (
    <Section id="s-ai" title="AI">
      <ModelRow
        title="質疑応答のモデル"
        help="動画と解説への質問、接続テストに使います。"
        label="質疑応答のモデル"
        value={models.chat}
        onSave={(model) => onSave("chat", model)}
      />
      <ModelRow
        title="記録・分析・問題生成のモデル"
        help="バックグラウンドのジョブすべてに使います。"
        label="記録・分析・問題生成のモデル"
        value={models.background}
        onSave={(model) => onSave("background", model)}
      />
      <ConnectionTestRow />
    </Section>
  );
}

// 保存できなかった理由。値が条件を満たさないとき（invalid_argument）は、バックエンドの説明を出す。
function saveFailureMessage(error: unknown): string {
  const commandError = asCommandError(error);
  if (commandError !== null && commandError.code === "invalid_argument") {
    return commandError.message;
  }
  return "設定を保存できませんでした。";
}

function ModelRow({
  title,
  help,
  label,
  value,
  onSave,
}: {
  title: string;
  help: string;
  label: string;
  value: string;
  onSave: (model: string) => Promise<void>;
}) {
  // 「その他」を選んだ状態か。保存済みの値がエイリアスでないときは、はじめから選んだ状態にする。
  const [other, setOther] = useState(!isAlias(value));
  const [text, setText] = useState(isAlias(value) ? "" : value);
  const [error, setError] = useState<string | null>(null);

  const save = (model: string) => {
    setError(null);
    onSave(model).catch((failure: unknown) => {
      console.error("モデルを保存できなかった", failure);
      setError(saveFailureMessage(failure));
      // 保存した値に戻す。
      setOther(!isAlias(value));
      setText(isAlias(value) ? "" : value);
    });
  };

  const submitOther = (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    commitOther();
  };
  const commitOther = () => {
    const trimmed = text.trim();
    if (trimmed !== value) {
      save(trimmed);
    }
  };

  return (
    <Row
      title={title}
      help={help}
      below={
        error !== null && (
          <p role="alert" className="settings-field-error">
            {error}
          </p>
        )
      }
    >
      <div className="pg-select-wrap">
        <select
          className="pg-select"
          aria-label={label}
          value={other ? OTHER : value}
          onChange={(event) => {
            const next = event.target.value;
            setError(null);
            if (next === OTHER) {
              setOther(true);
              setText(isAlias(value) ? "" : value);
            } else {
              setOther(false);
              if (next !== value) {
                save(next);
              }
            }
          }}
        >
          {ALIASES.map((alias) => (
            <option key={alias} value={alias}>
              {alias}
            </option>
          ))}
          <option value={OTHER}>その他（モデル ID を入力）</option>
        </select>
        <ChevronDown
          className="pg-icon"
          aria-hidden="true"
          size={16}
          strokeWidth={2.2}
        />
      </div>
      {other && (
        <form onSubmit={submitOther}>
          <input
            type="text"
            className="pg-input pg-input--mono"
            aria-label={`${label}のモデル ID`}
            placeholder="モデル ID（例: claude-opus-4-1）"
            value={text}
            onChange={(event) => {
              setText(event.target.value);
            }}
            onBlur={commitOther}
          />
        </form>
      )}
    </Row>
  );
}

type TestState =
  | { status: "idle" }
  | { status: "testing" }
  | { status: "done"; result: ConnectionTestResult }
  | { status: "unavailable" }
  | { status: "failed" };

function ConnectionTestRow() {
  const [state, setState] = useState<TestState>({ status: "idle" });

  const run = () => {
    setState({ status: "testing" });
    testClaudeConnection().then(
      (result) => {
        setState({ status: "done", result });
      },
      (error: unknown) => {
        if (asCommandError(error)?.code === "claude_unavailable") {
          setState({ status: "unavailable" });
        } else {
          console.error("接続テストを実行できなかった", error);
          setState({ status: "failed" });
        }
      },
    );
  };

  return (
    <Row
      title="接続テスト"
      help="短い指示を質疑応答のモデルで実行し、応答が返るかを確かめます。"
      below={<TestDetail state={state} />}
    >
      <TestStatus state={state} />
      <button
        type="button"
        className="pg-btn"
        disabled={state.status === "testing"}
        onClick={run}
      >
        接続テスト
      </button>
    </Row>
  );
}

function TestStatus({ state }: { state: TestState }) {
  switch (state.status) {
    case "idle":
      return null;
    case "testing":
      return (
        <span className="pg-ok-line" role="status">
          <LoaderCircle className="pg-icon pg-spin" {...ICON} />
          テスト中です
        </span>
      );
    case "done":
      if (state.result.ok) {
        return (
          <span className="pg-ok-line" role="status">
            <CircleCheck className="pg-icon" {...ICON} />
            応答がありました（{state.result.model}）
          </span>
        );
      }
      return (
        <span className="pg-ok-line" role="alert">
          <CircleAlert className="pg-icon" {...ICON} />
          {state.result.error === null
            ? "AI の呼び出しに失敗しました"
            : aiErrorMessage(state.result.error).message}
        </span>
      );
    case "unavailable":
      return (
        <span className="pg-ok-line" role="alert">
          <CircleAlert className="pg-icon" {...ICON} />
          Claude Code が見つかりません
          <button
            type="button"
            className="pg-btn"
            onClick={() => {
              openInstallGuide("claude").catch((error: unknown) => {
                console.error("Claude Code の導入手順を開けなかった", error);
              });
            }}
          >
            導入手順を見る
          </button>
        </span>
      );
    case "failed":
      return (
        <span className="pg-ok-line" role="alert">
          <CircleAlert className="pg-icon" {...ICON} />
          接続テストを実行できませんでした
        </span>
      );
  }
}

// 「その他」のエラーの詳細は、折りたたんで出す。
function TestDetail({ state }: { state: TestState }) {
  if (state.status !== "done" || state.result.error === null) {
    return null;
  }
  const { detail } = aiErrorMessage(state.result.error);
  if (detail === null) {
    return null;
  }
  return (
    <details className="settings-field-detail">
      <summary>詳細</summary>
      <pre>{detail}</pre>
    </details>
  );
}
