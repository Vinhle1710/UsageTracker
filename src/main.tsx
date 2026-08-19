import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { rootForWindow } from "./app/window-root";
import { EdgeTabApp, OverlayApp, SettingsApp } from "./app/roots";
import "./styles/app.css";

const label = (() => {
  try { return getCurrentWindow().label; } catch { return "main"; }
})();
const root = rootForWindow(label);
const App = root === "overlay" ? OverlayApp : root === "settings" ? SettingsApp : EdgeTabApp;
createRoot(document.getElementById("app")!).render(<StrictMode><App /></StrictMode>);
