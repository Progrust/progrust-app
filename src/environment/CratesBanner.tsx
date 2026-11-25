// 外部クレートの初回準備中・失敗のときの案内
// （documents/spec/exercise-runner/crates-and-workspace.md「準備中・失敗時の動作」、
// design-system/progrust/components/Banner/README.md）。not_started・ready のときは何も出さない。
import { CircleAlert, LoaderCircle } from "lucide-react";
import { retryCratePreparation } from "../commands/environment";
import type { CratesStatus } from "../commands/environment";

// where: 案内を出す画面。home はトップページ、exercise は許可クレートを使う演習の画面。
export function CratesBanner({
  crates,
  where,
  className,
}: {
  crates: CratesStatus | null;
  where: "home" | "exercise";
  className?: string;
}) {
  if (
    crates === null ||
    (crates.status !== "preparing" && crates.status !== "failed")
  ) {
    return null;
  }
  const base =
    crates.status === "preparing"
      ? "pg-banner pg-banner--info"
      : "pg-banner pg-banner--ng";
  const classes = className === undefined ? base : `${base} ${className}`;
  if (crates.status === "preparing") {
    return (
      <div className={classes} role="status">
        <LoaderCircle
          className="pg-icon pg-spin"
          aria-hidden="true"
          size={16}
          strokeWidth={2.2}
        />
        <p>
          <b>外部クレートを準備しています。</b>
          {where === "home"
            ? "外部クレートを使う演習は、準備が終わるまで実行・提出できません。ほかの演習はすぐに使えます。"
            : "この演習は準備が終わるまで実行・提出できません。"}
        </p>
      </div>
    );
  }
  return (
    <div className={classes} role="alert">
      <CircleAlert
        className="pg-icon"
        aria-hidden="true"
        size={16}
        strokeWidth={2.2}
      />
      <p>
        <b>外部クレートの準備に失敗しました。</b>
        ネットワーク接続を確認してください。
        {where === "home"
          ? "外部クレートを使わない演習は続けられます。"
          : "この演習は準備ができるまで実行・提出できません。"}
      </p>
      <button
        type="button"
        className="pg-btn pg-btn--sm pg-btn--outline"
        onClick={retry}
      >
        再試行
      </button>
    </div>
  );
}

// 初回準備をやり直す。始められなかったときは、画面には出さず、記録だけする。
function retry() {
  retryCratePreparation().catch((error: unknown) => {
    console.error("外部クレートの準備をやり直せなかった", error);
  });
}
