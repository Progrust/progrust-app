// 設定画面の「環境」の区分。決まりは documents/spec/screens/settings-screen.md「環境」、
// 配置は design-system/ui-design/Settings.dc.html。
import { CircleAlert, CircleCheck, LoaderCircle } from "lucide-react";
import { useState } from "react";
import type { ReactNode, SubmitEvent } from "react";
import { redetectTools, retryCratePreparation } from "../commands/environment";
import type { EnvironmentStatus } from "../commands/environment";
import type { Settings } from "../commands/settings";
import { useEnvironmentStatus } from "../environment";
import { openInstallGuide } from "../environment/installGuide";
import type { InstallGuideTool } from "../environment/installGuide";
import { Row, Section } from "./SettingsParts";

// この区分に行のあるツール。
type ToolKey = Extract<keyof Settings["tool_paths"], "cargo" | "claude">;

const ICON = { size: 14, strokeWidth: 2.4, "aria-hidden": true } as const;

// 準備できた日時を「10/28 19:02」の形（この PC の時刻）にする。読めない日時は、そのまま出す。
export function formatPreparedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const two = (n: number) => String(n).padStart(2, "0");
  return `${date.getMonth() + 1}/${date.getDate()} ${two(date.getHours())}:${two(date.getMinutes())}`;
}

// onSavePath: tool_paths を保存して、検出し直す。失敗したら reject する。
export function EnvironmentSection({
  toolPaths,
  onSavePath,
}: {
  toolPaths: Settings["tool_paths"];
  onSavePath: (tool: ToolKey, path: string | null) => Promise<void>;
}) {
  const status = useEnvironmentStatus();
  const [failure, setFailure] = useState<string | null>(null);

  const redetect = () => {
    setFailure(null);
    redetectTools().catch((error: unknown) => {
      console.error("cargo・claude を検出し直せなかった", error);
      setFailure("検出し直せませんでした。");
    });
  };
  const retry = () => {
    setFailure(null);
    retryCratePreparation().catch((error: unknown) => {
      console.error("外部クレートの準備をやり直せなかった", error);
      setFailure("外部クレートの準備をやり直せませんでした。");
    });
  };

  return (
    <Section id="s-env" title="環境">
      {failure !== null && <p role="alert">{failure}</p>}
      <ToolRow
        tool="cargo"
        name="Rust ツールチェーン"
        status={status}
        specified={toolPaths.cargo}
        onSavePath={onSavePath}
        onRedetect={redetect}
      />
      <ToolRow
        tool="claude"
        name="Claude Code"
        status={status}
        specified={toolPaths.claude}
        onSavePath={onSavePath}
        onRedetect={redetect}
      />
      <CratesRow status={status} onRetry={retry} />
    </Section>
  );
}

type Detected = {
  found: boolean;
  path: string | null;
  // 見つかったときの説明（バージョンなど）。
  detail: ReactNode;
};

function detectedOf(
  tool: ToolKey,
  status: EnvironmentStatus,
): Detected | "version_too_old" {
  if (tool === "cargo") {
    const { cargo } = status;
    if (cargo.status === "not_found") {
      return { found: false, path: null, detail: null };
    }
    if (cargo.status === "version_too_old") {
      return "version_too_old";
    }
    return {
      found: true,
      path: cargo.cargo_path,
      detail: `rustc ${cargo.rustc_version ?? "不明"}`,
    };
  }
  const { claude } = status;
  return {
    found: claude.status === "found",
    path: claude.path,
    detail: claude.version ?? "バージョン不明",
  };
}

function ToolRow({
  tool,
  name,
  status,
  specified,
  onSavePath,
  onRedetect,
}: {
  tool: ToolKey;
  name: string;
  status: EnvironmentStatus | null;
  specified: string | null;
  onSavePath: (tool: ToolKey, path: string | null) => Promise<void>;
  onRedetect: () => void;
}) {
  const [editing, setEditing] = useState(false);

  const buttons = (
    <>
      <button
        type="button"
        className="pg-btn"
        aria-label={`${name}のパスを指定`}
        aria-expanded={editing}
        onClick={() => {
          setEditing(!editing);
        }}
      >
        パスを指定
      </button>
      <button
        type="button"
        className="pg-btn"
        aria-label={`${name}を再検出`}
        onClick={onRedetect}
      >
        再検出
      </button>
    </>
  );
  const editor = editing ? (
    <PathEditor
      tool={tool}
      name={name}
      specified={specified}
      onSavePath={onSavePath}
      onClose={() => {
        setEditing(false);
      }}
    />
  ) : null;

  if (status === null) {
    return (
      <Row title={name} help="確認しています。" below={editor}>
        {buttons}
      </Row>
    );
  }

  const detected = detectedOf(tool, status);
  const info = detected === "version_too_old" ? null : detected;
  const detectedPath = info === null ? status.cargo.cargo_path : info.path;
  const found = info !== null && info.found;
  const tooOld = info === null;
  const hasPath = detectedPath !== null;
  // 指定したパスが使われたか。tool_paths に値があるのに、検出したパスがそれと違うか、見つからないときは使われていない。
  const specifiedUsed = specified !== null && detectedPath === specified;
  const specifiedUnused = specified !== null && !specifiedUsed;

  let help: ReactNode;
  if (tooOld) {
    help = (
      <>
        Rust のバージョンが古いです。今のバージョンは{" "}
        {status.cargo.rustc_version ?? "不明"}、必要なバージョンは{" "}
        {status.cargo.required_rustc_version} です。
        {hasPath && (
          <>
            {" "}
            <code>{detectedPath}</code>
            {specifiedUsed ? "（指定したパス）" : "（自動で検出）"}
          </>
        )}
      </>
    );
  } else if (info !== null && found && hasPath) {
    help = (
      <>
        {info.detail} ・ <code>{detectedPath}</code>
        {specifiedUsed ? "（指定したパス）" : "（自動で検出）"}
      </>
    );
  } else {
    help = "見つかりませんでした。";
  }

  const guide = !found && !tooOld;
  return (
    <Row
      title={name}
      help={
        <>
          {help}
          {specifiedUnused && (
            <>
              {" "}
              <span role="alert">
                指定したパス <code>{specified}</code>{" "}
                は使えませんでした。パスが正しいか確かめてください。
              </span>
            </>
          )}
        </>
      }
      below={editor}
    >
      {guide && (
        <button
          type="button"
          className="pg-btn"
          onClick={() => {
            openGuide(tool);
          }}
        >
          導入手順を見る
        </button>
      )}
      {buttons}
    </Row>
  );
}

function openGuide(tool: InstallGuideTool) {
  openInstallGuide(tool).catch((error: unknown) => {
    console.error("導入手順を開けなかった", error);
  });
}

// パスの入力欄。保存すると tool_paths を保存して検出し直す。空にして保存すると、自動検出に戻る。
function PathEditor({
  tool,
  name,
  specified,
  onSavePath,
  onClose,
}: {
  tool: ToolKey;
  name: string;
  specified: string | null;
  onSavePath: (tool: ToolKey, path: string | null) => Promise<void>;
  onClose: () => void;
}) {
  const [value, setValue] = useState(specified ?? "");
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  const submit = (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = value.trim();
    setSaving(true);
    setFailed(false);
    onSavePath(tool, trimmed === "" ? null : trimmed).then(
      () => {
        setSaving(false);
        onClose();
      },
      () => {
        setSaving(false);
        setFailed(true);
      },
    );
  };

  return (
    <form className="settings-path" onSubmit={submit}>
      <input
        type="text"
        className="pg-input pg-input--mono"
        aria-label={`${name}のパス`}
        placeholder="空にすると、自動で検出します（例: which の出力）"
        value={value}
        onChange={(event) => {
          setValue(event.target.value);
        }}
      />
      <button type="submit" className="pg-btn" disabled={saving}>
        保存
      </button>
      <button type="button" className="pg-btn" onClick={onClose}>
        閉じる
      </button>
      {failed && <p role="alert">保存できませんでした。</p>}
    </form>
  );
}

function CratesRow({
  status,
  onRetry,
}: {
  status: EnvironmentStatus | null;
  onRetry: () => void;
}) {
  const title = "外部クレートの準備";
  const help =
    "演習で使える外部クレートを、初回にダウンロードしてビルドしておきます。";
  if (status === null) {
    return (
      <Row title={title} help={help}>
        {null}
      </Row>
    );
  }
  const { crates } = status;
  switch (crates.status) {
    case "ready":
      return (
        <Row title={title} help={help}>
          <span className="pg-ok-line">
            <CircleCheck className="pg-icon" {...ICON} />
            準備できています
            {crates.prepared_at !== null &&
              `（${formatPreparedAt(crates.prepared_at)}）`}
          </span>
        </Row>
      );
    case "preparing":
      return (
        <Row title={title} help={help}>
          <span className="pg-ok-line">
            <LoaderCircle className="pg-icon pg-spin" {...ICON} />
            準備中です
          </span>
        </Row>
      );
    case "failed":
      return (
        <Row
          title={title}
          help={
            <>
              {help} 準備に失敗しました: {crates.error ?? "原因は不明です"}
            </>
          }
        >
          <span className="pg-ok-line">
            <CircleAlert className="pg-icon" {...ICON} />
            準備に失敗しました
          </span>
          <button type="button" className="pg-btn" onClick={onRetry}>
            再試行
          </button>
        </Row>
      );
    case "not_started":
      return (
        <Row
          title={title}
          help="Rust ツールチェーンが見つからないので、準備していません。導入して「再検出」すると、準備を始めます。"
        >
          {null}
        </Row>
      );
  }
}
