import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook, act } from "@testing-library/react";

import {
  getFeatureFlag,
  setFeatureFlag,
  useFeatureFlag,
} from "@/lib/featureFlags";

describe("featureFlags", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    setFeatureFlag("central.newLayout", null);
  });

  it("returns the default value when nothing is set", () => {
    // M6: V2 默认 ON。
    expect(getFeatureFlag("central.newLayout")).toBe(true);
  });

  it("setFeatureFlag(true) overrides the default", () => {
    setFeatureFlag("central.newLayout", true);
    expect(getFeatureFlag("central.newLayout")).toBe(true);
  });

  it("setFeatureFlag(false) explicitly disables", () => {
    setFeatureFlag("central.newLayout", false);
    expect(getFeatureFlag("central.newLayout")).toBe(false);
  });

  it("setFeatureFlag(null) clears the override", () => {
    setFeatureFlag("central.newLayout", false);
    setFeatureFlag("central.newLayout", null);
    expect(getFeatureFlag("central.newLayout")).toBe(true);
  });

  it("accepts diverse string spellings", () => {
    window.localStorage.setItem("featureFlag.central.newLayout", "ON");
    expect(getFeatureFlag("central.newLayout")).toBe(true);
    window.localStorage.setItem("featureFlag.central.newLayout", "yes");
    expect(getFeatureFlag("central.newLayout")).toBe(true);
    window.localStorage.setItem("featureFlag.central.newLayout", "no");
    expect(getFeatureFlag("central.newLayout")).toBe(false);
    window.localStorage.setItem("featureFlag.central.newLayout", "garbage");
    expect(getFeatureFlag("central.newLayout")).toBe(true); // fallback to default (M6: ON)
  });

  it("useFeatureFlag reflects current value and updates on change", () => {
    const { result } = renderHook(() => useFeatureFlag("central.newLayout"));
    // M6：默认 true。
    expect(result.current).toBe(true);

    act(() => {
      setFeatureFlag("central.newLayout", false);
    });
    expect(result.current).toBe(false);

    act(() => {
      setFeatureFlag("central.newLayout", null);
    });
    expect(result.current).toBe(true);
  });
});
