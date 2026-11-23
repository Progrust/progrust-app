// 画面の切り替え。画面は 6 つで、URL を持たないので、ルーターのライブラリは使わず、
// いまの画面を状態として持つ。
import { createContext, useContext } from "react";

export type Route =
  | { name: "home" }
  | { name: "lessons" }
  | { name: "video"; lessonId: string }
  | { name: "exercise"; lessonId: string }
  | { name: "generated" }
  | { name: "settings" };

// ヘッダーのメインメニューの項目。
export type MenuItem = "home" | "lessons" | "generated" | "settings";

// いまの画面で選んでおくメニューの項目。動画画面と演習画面は「レッスン」。
export function menuItemOf(route: Route): MenuItem {
  switch (route.name) {
    case "home":
      return "home";
    case "lessons":
    case "video":
    case "exercise":
      return "lessons";
    case "generated":
      return "generated";
    case "settings":
      return "settings";
  }
}

export type Navigation = {
  route: Route;
  navigate: (route: Route) => void;
};

export const NavigationContext = createContext<Navigation | null>(null);

// 画面から、いまの画面と、画面を切り替える関数を使う。
export function useNavigation(): Navigation {
  const navigation = useContext(NavigationContext);
  if (navigation === null) {
    throw new Error("NavigationContext の外で useNavigation を呼んだ");
  }
  return navigation;
}
