// 設定画面の区分と行（design-system/ui-design/Settings.dc.html の配置）。
import type { ReactNode } from "react";

export function Section({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="settings-sec" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      <div className="pg-settings">{children}</div>
    </section>
  );
}

export function Row({
  title,
  help,
  children,
  below,
}: {
  title: string;
  help: ReactNode;
  children: ReactNode;
  // 行の下に、幅いっぱいに出すもの（入力欄など）。
  below?: ReactNode;
}) {
  return (
    <div className="pg-settings-row">
      <div>
        <h3>{title}</h3>
        <p>{help}</p>
      </div>
      <div className="settings-ctrl">{children}</div>
      {below}
    </div>
  );
}
