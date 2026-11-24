import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";
import { lessonVideos } from "./tools/lesson-videos.ts";

export default defineConfig({
  plugins: [lessonVideos(), react()],
  // Tauri の出力（Rust のエラー）を消さない。
  clearScreen: false,
  // 開発時の起動で前もってまとめる依存（node_modules/.vite/deps）で、React を 1 つの塊に入れる。
  // 指定しないと、教材の動画のコードがあるときに React が 2 つの塊に重複して入り、
  // 画面が何も表示されない（ADR 0199）。本番ビルドには効かない。
  // まとめる依存を探す入口は、アプリの index.html だけにする。既定では、リポジトリの中の HTML をすべて入口にするので、
  // gitignore されたディレクトリに、自分の node_modules を持つ別の HTML があると、その React もまとめに入って
  // React が 2 つになり、画面が何も表示されない（ADR 0232）。
  optimizeDeps: {
    entries: ["index.html"],
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "react",
              test: /node_modules[\\/](react|react-dom|scheduler)[\\/]/,
            },
          ],
        },
      },
    },
  },
  server: {
    // tauri.conf.json の devUrl と同じポート。使えないときは失敗させる。
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: "app",
          include: ["src/**/*.test.{ts,tsx}"],
          environment: "jsdom",
          setupFiles: ["src/test-setup.ts"],
          // Vitest は CSS を空にして読み込む。エディタの配色のテストが、
          // デザインシステムのトークンの CSS の中身（?raw）を読めるようにする。
          css: { include: [/design-system\/progrust\/tokens\.css/] },
        },
      },
      {
        test: {
          name: "tools",
          include: ["tools/**/*.test.ts"],
          environment: "node",
        },
      },
    ],
  },
});
