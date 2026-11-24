// cargo が見つからないとき・rustc が古いときの案内
// （documents/spec/architecture/external-tools.md「見つからない場合」「バージョンが足りない場合」、
// design-system/progrust/components/Banner/README.md）。found のときは何も出さない。
import { TriangleAlert } from "lucide-react";
import type { CargoStatus } from "../commands/environment";

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
        <p>
          <b>Rust ツールチェーンが見つかりません。</b>
          実行・テスト・提出はできません。動画の視聴は続けられます。rustup
          の導入手順: https://rustup.rs
          を開き、案内に従って導入してから、アプリを開き直してください。
        </p>
      ) : (
        <p>
          <b>Rust のバージョンが古いです。</b>
          今のバージョンは {cargo.rustc_version ?? "不明"}
          、必要なバージョンは {
            cargo.required_rustc_version
          } です。ターミナルで <code>rustup update stable</code>{" "}
          を実行して更新してから、アプリを開き直してください。
        </p>
      )}
    </div>
  );
}
