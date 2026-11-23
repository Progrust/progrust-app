import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../design-system/progrust/tokens.css";
import "../design-system/progrust-video/tokens.css";
import "../design-system/progrust/components/bundle.css";
import "../design-system/progrust-video/components/bundle.css";
import { App } from "./App";
import { getSettings } from "./commands/settings";
import { startTheme } from "./theme";

const root = document.getElementById("root");
if (root === null) {
  throw new Error("#root が見つからない");
}

void startTheme(getSettings);

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
