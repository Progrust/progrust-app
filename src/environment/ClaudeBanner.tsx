// Claude Code が見つからないときの案内
// （documents/spec/architecture/external-tools.md「見つからない場合」、
// design-system/progrust/components/Banner/README.md）。found のときは何も出さない。
import { ExternalLink, TriangleAlert } from "lucide-react";
import type { ClaudeStatus } from "../commands/environment";
import { useNavigation } from "../router";
import { openInstallGuide } from "./installGuide";

export function ClaudeBanner({
  claude,
  className,
}: {
  claude: ClaudeStatus | null;
  className?: string;
}) {
  const { navigate } = useNavigation();
  if (claude === null || claude.status !== "not_found") {
    return null;
  }
  const classes =
    className === undefined ? "pg-banner" : `pg-banner ${className}`;
  return (
    <div className={classes} role="alert">
      <TriangleAlert
        className="pg-icon"
        aria-hidden="true"
        size={16}
        strokeWidth={2.2}
      />
      <p>
        <b>Claude Code が見つかりません。</b>
        質問・学習の分析・問題生成は使えません。レッスンは続けられます。
      </p>
      <button
        type="button"
        className="pg-btn pg-btn--sm pg-btn--outline"
        onClick={openClaudeGuide}
      >
        導入手順を見る
        <ExternalLink
          className="pg-icon"
          aria-hidden="true"
          size={14}
          strokeWidth={2.4}
        />
      </button>
      <button
        type="button"
        className="pg-btn pg-btn--sm pg-btn--outline"
        onClick={() => navigate({ name: "settings" })}
      >
        パスを指定
      </button>
    </div>
  );
}

// 導入手順を既定ブラウザで開く。開けなかったときは、画面には出さず、記録だけする。
function openClaudeGuide() {
  openInstallGuide("claude").catch((error: unknown) => {
    console.error("Claude Code の導入手順を開けなかった", error);
  });
}
