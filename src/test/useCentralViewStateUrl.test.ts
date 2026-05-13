import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

import { useCentralViewStateUrl } from "@/hooks/useCentralViewStateUrl";
import { defaultCentralViewState } from "@/lib/centralViewState";

function setUrl(search: string) {
  const url = `${window.location.pathname}${search}`;
  window.history.replaceState(null, "", url);
}

describe("useCentralViewStateUrl", () => {
  beforeEach(() => {
    setUrl("");
  });

  it("returns default state when URL has no params", () => {
    const { result } = renderHook(() => useCentralViewStateUrl());
    expect(result.current[0]).toEqual(defaultCentralViewState());
  });

  it("parses initial state from URL search params", () => {
    setUrl("?q=hello&repos=r1,r2&tags=t1&view=list&group=tag&sort=updatedAt:desc");
    const { result } = renderHook(() => useCentralViewStateUrl());
    const [state] = result.current;
    expect(state.q).toBe("hello");
    expect(state.repos).toEqual(["r1", "r2"]);
    expect(state.tags).toEqual(["t1"]);
    expect(state.view).toBe("list");
    expect(state.group).toBe("tag");
    expect(state.sortField).toBe("updatedAt");
    expect(state.sortDir).toBe("desc");
  });

  it("writes state back to URL on update", async () => {
    const { result } = renderHook(() => useCentralViewStateUrl());

    act(() => {
      const [, setState] = result.current;
      setState({ ...defaultCentralViewState(), q: "abc", repos: ["r1"] });
    });

    const url = new URL(window.location.href);
    expect(url.searchParams.get("q")).toBe("abc");
    expect(url.searchParams.get("repos")).toBe("r1");
  });

  it("setter accepts functional updater", () => {
    const { result } = renderHook(() => useCentralViewStateUrl());

    act(() => {
      const [, setState] = result.current;
      setState((prev) => ({ ...prev, q: "from-fn" }));
    });

    expect(result.current[0].q).toBe("from-fn");
    expect(new URL(window.location.href).searchParams.get("q")).toBe("from-fn");
  });

  it("disabled option skips URL writes", () => {
    const { result } = renderHook(() => useCentralViewStateUrl({ disabled: true }));

    act(() => {
      const [, setState] = result.current;
      setState({ ...defaultCentralViewState(), q: "silent" });
    });

    expect(result.current[0].q).toBe("silent");
    expect(new URL(window.location.href).searchParams.get("q")).toBeNull();
  });

  it("popstate event re-syncs state from URL", () => {
    const { result } = renderHook(() => useCentralViewStateUrl());

    act(() => {
      setUrl("?q=after-back&view=list");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });

    expect(result.current[0].q).toBe("after-back");
    expect(result.current[0].view).toBe("list");
  });

  it("empty default state clears URL search", () => {
    setUrl("?q=initial");
    const { result } = renderHook(() => useCentralViewStateUrl());
    expect(result.current[0].q).toBe("initial");

    act(() => {
      const [, setState] = result.current;
      setState(defaultCentralViewState());
    });

    expect(new URL(window.location.href).search).toBe("");
  });
});
