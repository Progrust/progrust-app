import { useEffect, useState } from "react";
import "./App.css";
import { getAppVersion } from "./commands/settings";

// 仮のページ。M1 で画面に置き換える。
export function App() {
  const version = useAppVersion();
  return (
    <main className="placeholder">
      <h1>Progrust</h1>
      <p>Rust プログラミング学習アプリ（準備中）</p>
      {version !== null && <p>バージョン {version}</p>}
    </main>
  );
}

// バックエンドから受け取ったバージョン。受け取るまでと、受け取れなかったときは null。
function useAppVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    getAppVersion()
      .then((result) => {
        if (active) {
          setVersion(result.version);
        }
      })
      .catch((error: unknown) => {
        console.error("get_app_version に失敗した", error);
      });
    return () => {
      active = false;
    };
  }, []);
  return version;
}
