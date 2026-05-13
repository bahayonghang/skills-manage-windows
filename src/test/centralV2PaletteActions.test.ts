import { describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import type { TFunction } from "i18next";

import { useCentralV2PaletteActions } from "../pages/centralV2PaletteActions";
import {
  defaultCentralViewState,
  type CentralViewState,
} from "../lib/centralViewState";

function fakeT(key: string, params?: Record<string, unknown>): string {
  if (params && typeof params.label === "string") return `${key}::${params.label}`;
  return key;
}

const groupByOptions = [
  { value: "none" as const, label: "None" },
  { value: "repository" as const, label: "Repository" },
  { value: "owner" as const, label: "Owner" },
  { value: "tag" as const, label: "Tag" },
  { value: "status" as const, label: "Status" },
];

function setup(overrides: {
  viewState?: CentralViewState;
  canSaveCurrent?: boolean;
} = {}) {
  const setViewState = vi.fn();
  const onSaveCurrentView = vi.fn();
  const onCreateTagGroup = vi.fn();
  const onSwitchToClassic = vi.fn();

  const { result } = renderHook(() =>
    useCentralV2PaletteActions({
      t: fakeT as unknown as TFunction,
      viewState: overrides.viewState ?? defaultCentralViewState(),
      setViewState,
      canSaveCurrent: overrides.canSaveCurrent ?? true,
      onSaveCurrentView,
      onCreateTagGroup,
      onSwitchToClassic,
      groupByOptions,
    }),
  );

  return {
    actions: result.current.actions,
    groupByOptions: result.current.groupByOptions,
    setViewState,
    onSaveCurrentView,
    onCreateTagGroup,
    onSwitchToClassic,
  };
}

describe("useCentralV2PaletteActions", () => {
  it("includes save-current when canSaveCurrent is true", () => {
    const { actions } = setup({ canSaveCurrent: true });
    expect(actions.some((a) => a.id === "save-current-view")).toBe(true);
  });

  it("omits save-current when canSaveCurrent is false", () => {
    const { actions } = setup({ canSaveCurrent: false });
    expect(actions.some((a) => a.id === "save-current-view")).toBe(false);
  });

  it("always includes create-tag-group and switch-to-classic", () => {
    const { actions } = setup();
    expect(actions.some((a) => a.id === "create-tag-group")).toBe(true);
    expect(actions.some((a) => a.id === "switch-to-classic")).toBe(true);
  });

  it("emits group-by entries for all modes except current", () => {
    const state = { ...defaultCentralViewState(), group: "tag" as const };
    const { actions } = setup({ viewState: state });
    const groupIds = actions.filter((a) => a.id.startsWith("group-by-")).map((a) => a.id);
    expect(groupIds).toEqual([
      "group-by-none",
      "group-by-repository",
      "group-by-owner",
      "group-by-status",
    ]);
  });

  it("group-by action sets viewState to target mode", () => {
    const state = defaultCentralViewState();
    const { actions, setViewState } = setup({ viewState: state });
    const repoAction = actions.find((a) => a.id === "group-by-repository");
    repoAction?.onSelect();
    expect(setViewState).toHaveBeenCalledWith({ ...state, group: "repository" });
  });

  it("save-current passes the default name placeholder", () => {
    const { actions, onSaveCurrentView } = setup({ canSaveCurrent: true });
    actions.find((a) => a.id === "save-current-view")?.onSelect();
    expect(onSaveCurrentView).toHaveBeenCalledWith("central.v2.savedViewsNamePlaceholder");
  });

  it("switch-to-classic calls onSwitchToClassic", () => {
    const { actions, onSwitchToClassic } = setup();
    actions.find((a) => a.id === "switch-to-classic")?.onSelect();
    expect(onSwitchToClassic).toHaveBeenCalledOnce();
  });

  it("create-tag-group calls onCreateTagGroup", () => {
    const { actions, onCreateTagGroup } = setup();
    actions.find((a) => a.id === "create-tag-group")?.onSelect();
    expect(onCreateTagGroup).toHaveBeenCalledOnce();
  });

  it("derives default groupByOptions from t when not provided", () => {
    const setViewState = vi.fn();
    const { result } = renderHook(() =>
      useCentralV2PaletteActions({
        t: fakeT as unknown as TFunction,
        viewState: defaultCentralViewState(),
        setViewState,
        canSaveCurrent: false,
        onSaveCurrentView: vi.fn(),
        onCreateTagGroup: vi.fn(),
        onSwitchToClassic: vi.fn(),
      }),
    );
    const values = result.current.groupByOptions.map((o) => o.value);
    expect(values).toEqual(["none", "repository", "owner", "tag", "status"]);
  });
});
