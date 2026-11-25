// cargo が見つからないとき・rustc が古いときの案内
// （documents/spec/architecture/external-tools.md「見つからない場合」「バージョンが足りない場合」、
// design-system/progrust/components/Banner/README.md）。found のときは何も出さない。
import { TriangleAlert } from "lucide-react";
import type { CargoStatus } from "../commands/environment";
import { openInstallGuide } from "./installGuide";

export function CargoBanner({
  cargo,
  className,
}: {
  cargo: CargoStatus | null;
  className?: string;
}) {
  if (cargo === null || cargo.status === "found") {
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
      {cargo.status === "not_found" ? (
        <>
          <p>
            <b>Rust ツールチェーンが見つかりません。</b>
            実行・テスト・提出はできません。動画の視聴は続けられます。rustup
            の導入手順に従って導入してから、設定画面の「環境」で「再検出」してください。
          </p>
          <button
            type="button"
            className="pg-btn pg-btn--sm pg-btn--outline"
            onClick={openRustupGuide}
          >
            rustup の導入手順
          </button>
        </>
      ) : (
        <p>
          <b>Rust のバージョンが古いです。</b>
          今のバージョンは {cargo.rustc_version ?? "不明"}
          、必要なバージョンは {
            cargo.required_rustc_version
          } です。ターミナルで <code>rustup update stable</code>{" "}
          を実行して更新してから、設定画面の「環境」で「再検出」してください。
        </p>
      )}
    </div>
  );
}

// rustup の導入手順を既定ブラウザで開く。開けなかったときは、画面には出さず、記録だけする。
function openRustupGuide() {
  openInstallGuide("cargo").catch((error: unknown) => {
    console.error("rustup の導入手順を開けなかった", error);
  });
}
