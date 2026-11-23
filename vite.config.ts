import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";
import { lessonVideos } from "./tools/lesson-videos.ts";

export default defineConfig({
  plugins: [lessonVideos(), react()],
  // Tauri の出力（Rust のエラー）を消さない。
  clearScreen: false,
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
