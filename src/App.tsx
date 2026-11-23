import { Hourglass } from "lucide-react";
import { useMemo, useState } from "react";
import "./App.css";
import { AppHeader } from "./AppHeader";
import { HomeScreen } from "./home/HomeScreen";
import { LessonsScreen } from "./lessons/LessonsScreen";
import { NavigationContext } from "./router";
import type { Route } from "./router";
import { SettingsScreen } from "./settings/SettingsScreen";
import { VideoScreen } from "./video/VideoScreen";

// 画面全体がスクロールしない画面。ほかの画面は、縦に長いページとしてスクロールする。
function isFixedHeight(route: Route): boolean {
  return (
    route.name === "home" || route.name === "video" || route.name === "exercise"
  );
}

export function App() {
  const [route, setRoute] = useState<Route>({ name: "home" });
  const navigation = useMemo(() => ({ route, navigate: setRoute }), [route]);
  return (
    <NavigationContext value={navigation}>
      <div
        className={`app ${isFixedHeight(route) ? "app-window" : "app-page"}`}
      >
        <AppHeader />
        <Screen route={route} />
      </div>
    </NavigationContext>
  );
}

// いまの画面。各画面の中身は、後のタスクで作る。
function Screen({ route }: { route: Route }) {
  switch (route.name) {
    case "home":
      return <HomeScreen />;
    case "lessons":
      return <LessonsScreen />;
    case "video":
      // レッスンが変わったら（「次のレッスンへ」）、状態を作り直す。
      return <VideoScreen key={route.lessonId} lessonId={route.lessonId} />;
    case "exercise":
      return <Placeholder title="演習画面" detail={route.lessonId} preparing />;
    case "generated":
      return <Placeholder title="生成問題一覧" preparing />;
    case "settings":
      return <SettingsScreen />;
  }
}

function Placeholder({
  title,
  detail,
  preparing = false,
}: {
  title: string;
  detail?: string;
  preparing?: boolean;
}) {
  return (
    <main className="app-main">
      <div className="app-placeholder">
        <Hourglass aria-hidden="true" size={18} strokeWidth={2.2} />
        <h1 className="app-placeholder-title">{title}</h1>
        {detail !== undefined && (
          <p className="app-placeholder-note">{detail}</p>
        )}
        <p className="app-placeholder-note">
          {preparing ? "準備中" : "仮の画面"}
        </p>
      </div>
    </main>
  );
}
