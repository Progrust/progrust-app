// テーマの決定と反映（documents/spec/screens/appearance.md「テーマ」）。
import { useSyncExternalStore } from "react";
import type { Theme } from "./commands/settings";

const DARK_QUERY = "(prefers-color-scheme: dark)";

// 設定の theme と OS の設定から、実際のテーマを決める。
export function resolveTheme(
  theme: Theme,
  osIsDark: boolean,
): "light" | "dark" {
  if (theme === "system") {
    return osIsDark ? "dark" : "light";
  }
  return theme;
}

let current: Theme = "system";
let media: MediaQueryList | null = null;

function apply() {
  const osIsDark = media?.matches ?? false;
  document.documentElement.dataset.theme = resolveTheme(current, osIsDark);
}

// 設定の theme を反映する。ルート要素の data-theme を付け、system のときは OS の設定の変化に追従する。
// 設定を変えたときも、これを呼べばすぐ反映される。
export function applyTheme(theme: Theme): void {
  current = theme;
  if (media === null && typeof window.matchMedia === "function") {
    media = window.matchMedia(DARK_QUERY);
    media.addEventListener("change", apply);
  }
  apply();
}

// 起動時に、設定を読んでテーマを反映する。読むまでは、OS の設定に合わせる。
export async function startTheme(
  getSettings: () => Promise<{ theme: Theme }>,
): Promise<void> {
  applyTheme("system");
  try {
    applyTheme((await getSettings()).theme);
  } catch (error: unknown) {
    console.error("get_settings に失敗した", error);
  }
}

// いまの画面のテーマ（ルート要素の data-theme）。付いていなければライト。
export function currentTheme(): "light" | "dark" {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

function subscribeTheme(onChange: () => void): () => void {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
  return () => observer.disconnect();
}

// いまの画面のテーマを返し、テーマが変わると描画し直す（OS の設定に追従したときも）。
// CSS 変数では色を変えられない部品（Monaco Editor など）が使う。
export function useCurrentTheme(): "light" | "dark" {
  return useSyncExternalStore(subscribeTheme, currentTheme);
}

// テスト用。追従を止め、状態を戻す。
export function resetTheme(): void {
  media?.removeEventListener("change", apply);
  media = null;
  current = "system";
  delete document.documentElement.dataset.theme;
}
