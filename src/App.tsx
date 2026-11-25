import { Hourglass } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import "./App.css";
import { AppHeader, BareHeader } from "./AppHeader";
import { getSettings, updateSettings } from "./commands/settings";
import type { Settings } from "./commands/settings";
import { ExerciseScreen } from "./exercise/ExerciseScreen";
import { HomeScreen } from "./home/HomeScreen";
import { LessonsScreen } from "./lessons/LessonsScreen";
import { OnboardingScreen } from "./onboarding/OnboardingScreen";
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

// 起動時の画面の選び方（documents/spec/screens/first-launch.md）。
// 設定の first_launch_completed が false（settings.json が無いときを含む）なら初回起動画面、true ならトップページ。
// 設定を読めなかったときは、アプリを使えなくならないよう、トップページを表示する。
export function App() {
  const [settings, setSettings] = useState<Settings | "loading" | "failed">(
    "loading",
  );
  useEffect(() => {
    let cancelled = false;
    getSettings().then(
      (loaded) => {
        if (!cancelled) {
          setSettings(loaded);
        }
      },
      (error: unknown) => {
        console.error("get_settings に失敗した", error);
        if (!cancelled) {
          setSettings("failed");
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, []);
  if (settings === "loading") {
    return <div className="app app-page" />;
  }
  if (settings !== "failed" && !settings.first_launch_completed) {
    const current = settings;
    return (
      <div className="app app-page">
        <BareHeader />
        <OnboardingScreen
          onStart={async () => {
            const saved = await updateSettings({
              ...current,
              first_launch_completed: true,
            });
            setSettings(saved);
          }}
        />
      </div>
    );
  }
  return <MainApp />;
}

function MainApp() {
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
      // レッスンが変わったら、前のレッスンのコードと表示を残さないよう、状態を作り直す。
      return <ExerciseScreen key={route.lessonId} lessonId={route.lessonId} />;
    case "generated":
      return <Placeholder title="生成問題一覧" preparing />;
    case "settings":
      return <SettingsScreen />;
  }
}

function Placeholder({
  title,
  preparing = false,
}: {
  title: string;
  preparing?: boolean;
}) {
  return (
    <main className="app-main">
      <div className="app-placeholder">
        <Hourglass aria-hidden="true" size={18} strokeWidth={2.2} />
        <h1 className="app-placeholder-title">{title}</h1>
        <p className="app-placeholder-note">
          {preparing ? "準備中" : "仮の画面"}
        </p>
      </div>
    </main>
  );
}
