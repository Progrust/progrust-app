// ヘッダー（design-system/progrust/components/AppHeader/README.md）。
// 検索窓とジョブ数は、後のマイルストーンで足す。
import type { MouseEvent } from "react";
import { menuItemOf, useNavigation } from "./router";
import type { MenuItem, Route } from "./router";

const MENU: { item: MenuItem; label: string; route: Route }[] = [
  { item: "home", label: "ホーム", route: { name: "home" } },
  { item: "lessons", label: "レッスン", route: { name: "lessons" } },
  { item: "generated", label: "問題生成", route: { name: "generated" } },
  { item: "settings", label: "設定", route: { name: "settings" } },
];

// ロゴだけのヘッダー。初回起動画面で使う（メインメニュー・レッスン検索・ジョブの表示は出さない）。
export function BareHeader() {
  return (
    <header className="pg-header">
      <div className="pg-logo">Progrust</div>
    </header>
  );
}

export function AppHeader() {
  const { route, navigate } = useNavigation();
  const current = menuItemOf(route);
  return (
    <header className="pg-header">
      <div className="pg-logo">Progrust</div>
      <nav className="pg-nav" aria-label="メインメニュー">
        {MENU.map(({ item, label, route: target }) => (
          <a
            key={item}
            href="#"
            aria-current={item === current ? "page" : undefined}
            onClick={(event: MouseEvent) => {
              event.preventDefault();
              navigate(target);
            }}
          >
            {label}
          </a>
        ))}
      </nav>
    </header>
  );
}
