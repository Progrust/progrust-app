// 初回起動画面。決まりは documents/spec/screens/first-launch.md、
// 画面の配置は design-system/ui-design/Onboarding.dc.html。
import {
  ArrowRight,
  CircleAlert,
  CircleCheck,
  ExternalLink,
  Info,
  ListChecks,
  LoaderCircle,
  Play,
  RefreshCw,
  Terminal,
  TriangleAlert,
} from "lucide-react";
import { useState } from "react";
import type { ReactNode } from "react";
import { redetectTools, retryCratePreparation } from "../commands/environment";
import type { EnvironmentStatus } from "../commands/environment";
import { useEnvironmentStatus } from "../environment";
import { openInstallGuide } from "../environment/installGuide";
import type { InstallGuideTool } from "../environment/installGuide";
import "./onboarding.css";

const ICON = { size: 18, strokeWidth: 2.2, "aria-hidden": true } as const;

// onStart: 「始める」を押したときに呼ぶ。保存に失敗したら reject し、画面に残る。
export function OnboardingScreen({
  onStart,
}: {
  onStart: () => Promise<void>;
}) {
  const status = useEnvironmentStatus();
  const [starting, setStarting] = useState(false);
  const [startFailed, setStartFailed] = useState(false);

  function start() {
    setStarting(true);
    setStartFailed(false);
    onStart().catch((error: unknown) => {
      console.error("初回起動の完了を保存できなかった", error);
      setStartFailed(true);
      setStarting(false);
    });
  }

  return (
    <main className="pgx-onb">
      <section className="pgx-onb-lead">
        <p className="pgx-onb-lead-kicker">はじめに</p>
        <h1>Progrust で Rust を学ぶ</h1>
        <p>
          Rust の文法や考え方を、要素ごとに「動画で学ぶ →
          演習で書いて確かめる」の順に進めます。ほかのプログラミング言語を使ったことがある人に向けた教材です。
        </p>
      </section>

      <section className="pgx-onb-sec" aria-label="学習の流れ">
        <div className="pgx-flow">
          <FlowStep
            icon={<Play size={16} strokeWidth={2.2} aria-hidden />}
            title="動画で学ぶ"
          >
            ナレーションつきの短い動画で、1
            つの要素の考え方とコードの書き方を確かめます。
          </FlowStep>
          <FlowStep
            icon={<Terminal size={16} strokeWidth={2.2} aria-hidden />}
            title="演習で解く"
          >
            エディタでコードを書き、実行して動きを見てから、提出して正しいかを判定します。
          </FlowStep>
          <FlowStep
            icon={<ListChecks size={16} strokeWidth={2.2} aria-hidden />}
            title="AI が記録・分析する"
          >
            演習ごとの解き方を AI
            が記録し、苦手な要素を中心にした問題を作ります。
          </FlowStep>
        </div>
      </section>

      <section className="pgx-onb-sec" aria-labelledby="onb-ai">
        <h2 className="pgx-onb-h2" id="onb-ai">
          AI の機能と、送信されるデータ
        </h2>
        <div className="pgx-notice">
          <Info className="pg-icon" {...ICON} />
          <div>
            <p>
              質問への回答・学習記録・分析・問題生成には Claude Code
              を使います。このとき、
              <b>
                書いたコード・質問の内容・学習記録の内容が、Claude Code を通じて
                Anthropic に送信されます。
              </b>
            </p>
            <p>
              学習記録と分析ファイルはこのコンピューターに保存し、設定画面からいつでも開いたり、リセットしたりできます。AI
              の機能が使えなくても、レッスンは進められます。
            </p>
          </div>
        </div>
      </section>

      <section className="pgx-onb-sec" aria-labelledby="onb-env">
        <div className="pgx-onb-row">
          <h2 className="pgx-onb-h2" id="onb-env">
            環境の確認
          </h2>
          <button
            type="button"
            className="pg-btn pg-btn--sm pgx-onb-redetect"
            onClick={redetect}
          >
            <RefreshCw
              className="pg-icon"
              aria-hidden
              size={14}
              strokeWidth={2.4}
            />
            再検出
          </button>
        </div>
        <EnvList status={status} />
      </section>

      <div className="pgx-onb-foot">
        <p>
          見つからないツールがあっても先に進めます。
          <br />
          あとから設定画面の「環境」で確かめられます。
        </p>
        <button
          type="button"
          className="pg-btn pg-btn--accent pg-btn--xl"
          disabled={starting}
          onClick={start}
        >
          始める
          <ArrowRight
            className="pg-icon"
            aria-hidden
            size={16}
            strokeWidth={2.4}
          />
        </button>
      </div>
      {startFailed && (
        <p className="pgx-onb-error" role="alert">
          設定を保存できませんでした。もう一度「始める」を押してください。
        </p>
      )}
    </main>
  );
}

function FlowStep({
  icon,
  title,
  children,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="pgx-flow-step">
      <div className="pgx-onb-row">
        <span className="pgx-flow-icon">{icon}</span>
        {title}
      </div>
      <p>{children}</p>
    </div>
  );
}

// 検出し直す。表示は environment_updated で更新される。失敗したときは、記録だけする。
function redetect() {
  redetectTools().catch((error: unknown) => {
    console.error("cargo・claude を検出し直せなかった", error);
  });
}

function retry() {
  retryCratePreparation().catch((error: unknown) => {
    console.error("外部クレートの準備をやり直せなかった", error);
  });
}

function openGuide(tool: InstallGuideTool) {
  openInstallGuide(tool).catch((error: unknown) => {
    console.error("導入手順を開けなかった", error);
  });
}

type Tone = "ok" | "ng" | "error" | "busy";

function EnvRow({
  tone,
  name,
  detail,
  action,
}: {
  tone: Tone;
  name: string;
  detail: ReactNode;
  action?: ReactNode;
}) {
  const className = `pg-icon is-${tone}${tone === "busy" ? " pg-spin" : ""}`;
  return (
    <li>
      {tone === "ok" && <CircleCheck className={className} {...ICON} />}
      {tone === "ng" && <TriangleAlert className={className} {...ICON} />}
      {tone === "error" && <CircleAlert className={className} {...ICON} />}
      {tone === "busy" && <LoaderCircle className={className} {...ICON} />}
      <div>
        <p className="pg-env-name">{name}</p>
        <p className="pg-env-detail">{detail}</p>
      </div>
      {action}
    </li>
  );
}

function GuideButton({ tool }: { tool: InstallGuideTool }) {
  return (
    <button
      type="button"
      className="pg-btn pg-btn--sm"
      onClick={() => {
        openGuide(tool);
      }}
    >
      導入手順を見る
      <ExternalLink
        className="pg-icon"
        aria-hidden
        size={14}
        strokeWidth={2.4}
      />
    </button>
  );
}

function EnvList({ status }: { status: EnvironmentStatus | null }) {
  return (
    <ul className="pg-env">
      <CargoRow status={status} />
      <ClaudeRow status={status} />
      <CratesRow status={status} />
    </ul>
  );
}

function CargoRow({ status }: { status: EnvironmentStatus | null }) {
  const name = "Rust ツールチェーン";
  if (status === null) {
    return <EnvRow tone="busy" name={name} detail="確認しています。" />;
  }
  const { cargo } = status;
  if (cargo.status === "found") {
    return (
      <EnvRow
        tone="ok"
        name={name}
        detail={
          <>
            rustc {cargo.rustc_version ?? "不明"} ・{" "}
            <code>{cargo.cargo_path}</code>
          </>
        }
        action={<span className="pg-ok-line">使えます</span>}
      />
    );
  }
  if (cargo.status === "not_found") {
    return (
      <EnvRow
        tone="ng"
        name={name}
        detail="見つかりませんでした。導入するまで、演習の実行・テスト・提出は使えません。動画の視聴は続けられます。"
        action={<GuideButton tool="cargo" />}
      />
    );
  }
  return (
    <EnvRow
      tone="ng"
      name={name}
      detail={
        <>
          rustc {cargo.rustc_version ?? "不明"} が見つかりました。
          {cargo.required_rustc_version} 以上が必要です。ターミナルで{" "}
          <code>rustup update stable</code> を実行して更新してください。
        </>
      }
    />
  );
}

function ClaudeRow({ status }: { status: EnvironmentStatus | null }) {
  const name = "Claude Code";
  if (status === null) {
    return <EnvRow tone="busy" name={name} detail="確認しています。" />;
  }
  const { claude } = status;
  if (claude.status === "found") {
    return (
      <EnvRow
        tone="ok"
        name={name}
        detail={
          <>
            {claude.version === null ? "" : `${claude.version} ・ `}
            <code>{claude.path}</code>
          </>
        }
        action={<span className="pg-ok-line">使えます</span>}
      />
    );
  }
  return (
    <EnvRow
      tone="ng"
      name={name}
      detail="見つかりませんでした。導入するまで、質問・学習の分析・問題生成は使えません。"
      action={<GuideButton tool="claude" />}
    />
  );
}

function CratesRow({ status }: { status: EnvironmentStatus | null }) {
  const name = "外部クレートの準備";
  if (status === null) {
    return <EnvRow tone="busy" name={name} detail="確認しています。" />;
  }
  switch (status.crates.status) {
    case "preparing":
      return (
        <EnvRow
          tone="busy"
          name={name}
          detail="演習で使う外部クレートをダウンロードして、ビルドしています。準備が終わるまで、外部クレートを使う演習は実行できません。"
          action={<span className="pg-ok-line">準備中</span>}
        />
      );
    case "ready":
      return (
        <EnvRow
          tone="ok"
          name={name}
          detail="外部クレートを使う演習も実行できます。"
          action={<span className="pg-ok-line">準備できました</span>}
        />
      );
    case "failed":
      return (
        <EnvRow
          tone="error"
          name={name}
          detail="準備に失敗しました。ネットワーク接続を確認してください。外部クレートを使わない演習は使えます。"
          action={
            <button type="button" className="pg-btn pg-btn--sm" onClick={retry}>
              再試行
            </button>
          }
        />
      );
    case "not_started":
      return (
        <EnvRow
          tone="ng"
          name={name}
          detail="Rust ツールチェーンが見つからないので、準備していません。導入して「再検出」を押すと、準備を始めます。"
        />
      );
  }
}
