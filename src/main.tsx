import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { Toaster } from "sonner";
import App from "./App";
import "./index.css";
// Initialize i18n before rendering the app
import "./i18n";
// Initialize Catppuccin theme before rendering so there's no flash
import { useThemeStore } from "./stores/themeStore";
import {
  DEFAULT_FONT_PREFERENCES,
  applyFontPreferences,
  loadFontPreferences,
} from "./lib/displayFont";
import { installRuntimeLogger } from "./lib/runtimeLogger";
import { isTauriRuntime } from "@/lib/ipc";
import { installBrowserIpcFixtures } from "./fixtures";

// 浏览器演示态：先按命令名注册 IPC fixtures，再做任何会触发 invoke 的初始化
if (!isTauriRuntime()) {
  installBrowserIpcFixtures();
}

// Apply theme synchronously before React renders to prevent flash of wrong theme
useThemeStore.getState().init();
// 字体偏好：先 apply 默认值避免 layout 跳动；Tauri 端 IPC 完成后再覆盖
applyFontPreferences(DEFAULT_FONT_PREFERENCES);
void loadFontPreferences().then(applyFontPreferences);
installRuntimeLogger();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
      <Toaster position="bottom-right" richColors />
    </BrowserRouter>
  </React.StrictMode>,
);
