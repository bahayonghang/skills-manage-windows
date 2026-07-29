import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { Toaster } from "sonner";
import App from "./App";
import { StartupGate } from "@/components/startup/StartupGate";
import "./index.css";
// Initialize i18n before rendering the app
import "./i18n";
// Initialize Catppuccin theme before rendering so there's no flash
import {
  fontThemeModeForFlavor,
  useThemeStore,
} from "./stores/themeStore";
import {
  DEFAULT_THEMED_FONT_PREFERENCES,
  activateFontTheme,
  applyThemedFontPreferences,
  loadThemedFontPreferences,
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
let activeFontMode = fontThemeModeForFlavor(useThemeStore.getState().flavor);
// 字体偏好：先 apply 当前 mode 默认值避免 layout 跳动；IPC 完成后再覆盖
applyThemedFontPreferences(DEFAULT_THEMED_FONT_PREFERENCES, activeFontMode);
useThemeStore.subscribe((state) => {
  const nextMode = fontThemeModeForFlavor(state.flavor);
  if (nextMode === activeFontMode) return;
  activeFontMode = nextMode;
  activateFontTheme(nextMode);
});
void loadThemedFontPreferences().then((preferences) => {
  activeFontMode = fontThemeModeForFlavor(useThemeStore.getState().flavor);
  applyThemedFontPreferences(preferences, activeFontMode);
});
installRuntimeLogger();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <StartupGate>
        <App />
      </StartupGate>
      <Toaster position="bottom-right" richColors />
    </BrowserRouter>
  </React.StrictMode>,
);
