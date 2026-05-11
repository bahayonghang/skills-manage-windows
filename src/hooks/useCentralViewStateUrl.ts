/**
 * useCentralViewStateUrl — 把 `CentralViewState` 与 URL search 双向同步。
 *
 * 单一真相源是 React state。hook 负责：
 *   - 启动时读取 `location.search`，覆盖初始 state（如果 URL 有值）
 *   - state 变化时，把序列化结果写回 URL（`history.replaceState`，不污染历史）
 *   - 浏览器前进/后退（`popstate`）时，重新解析 URL → setState
 *
 * 仅在 Tauri 的 WebView 与浏览器都可工作。SSR / 无 window 环境下退化为纯
 * useState。
 */

import { useCallback, useEffect, useRef, useState } from "react";

import {
  defaultCentralViewState,
  parseCentralViewState,
  serializeCentralViewState,
  type CentralViewState,
} from "@/lib/centralViewState";

function hasWindow(): boolean {
  return typeof window !== "undefined" && typeof window.history !== "undefined";
}

function readFromUrl(): CentralViewState {
  if (!hasWindow()) return defaultCentralViewState();
  const params = new URLSearchParams(window.location.search);
  return parseCentralViewState(params);
}

function writeToUrl(state: CentralViewState): void {
  if (!hasWindow()) return;
  const search = serializeCentralViewState(state).toString();
  const url = `${window.location.pathname}${search ? `?${search}` : ""}${window.location.hash}`;
  window.history.replaceState(window.history.state, "", url);
}

export interface UseCentralViewStateUrlOptions {
  /** 关闭 URL 同步（用于单元测试或临时禁用）。 */
  disabled?: boolean;
  /** 初始 state 的覆盖来源。默认从 URL 读取。 */
  initial?: CentralViewState;
}

export function useCentralViewStateUrl(
  options: UseCentralViewStateUrlOptions = {},
): [CentralViewState, (next: CentralViewState | ((prev: CentralViewState) => CentralViewState)) => void] {
  const { disabled = false, initial } = options;
  const [state, setStateRaw] = useState<CentralViewState>(() => initial ?? readFromUrl());

  // 防止 popstate 触发后又把同样的 URL 写回去导致无限循环
  const skipNextWriteRef = useRef(false);

  const setState = useCallback(
    (next: CentralViewState | ((prev: CentralViewState) => CentralViewState)) => {
      setStateRaw((prev) => {
        const value = typeof next === "function" ? (next as (p: CentralViewState) => CentralViewState)(prev) : next;
        return value;
      });
    },
    [],
  );

  // state → URL
  useEffect(() => {
    if (disabled) return;
    if (skipNextWriteRef.current) {
      skipNextWriteRef.current = false;
      return;
    }
    writeToUrl(state);
  }, [disabled, state]);

  // 监听 popstate（浏览器前进/后退）
  useEffect(() => {
    if (disabled || !hasWindow()) return;
    const onPop = () => {
      const fromUrl = readFromUrl();
      skipNextWriteRef.current = true; // 避免下一轮 effect 又把它写回去
      setStateRaw(fromUrl);
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, [disabled]);

  return [state, setState];
}
