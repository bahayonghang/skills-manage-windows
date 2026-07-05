/**
 * 浏览器演示态安全网：删掉 __TAURI_INTERNALS__/__TAURI__ 模拟纯浏览器环境，
 * 安装 installBrowserIpcFixtures 后驱动 batch-1 各 store 的真实加载路径，
 * 断言 state 被 fixture 数据填充且 error === null。
 *
 * 若某 store 新增了未注册 fixture 的 invoke，dispatchIpcFixture 会 reject
 * IpcFixtureMissingError，对应断言在这里 fail loud —— 这正是安全网的用途。
 */
import { describe, it, expect, beforeAll, afterAll } from "vitest";

import { installBrowserIpcFixtures } from "@/fixtures";
import { clearIpcFixturesForTest } from "@/lib/ipc";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useRuntimeLogStore } from "@/stores/runtimeLogStore";
import { useSavedViewsStore } from "@/stores/savedViewsStore";
import { useSkillStore } from "@/stores/skillStore";
import { useTagGroupsStore } from "@/stores/tagGroupsStore";
import { useTargetStore } from "@/stores/targetStore";
import { useUsageStore } from "@/stores/usageStore";

const DESKTOP_ONLY_UNINSTALL_ERROR =
  "Uninstalling skills requires the Tauri desktop runtime.";

type TauriGlobals = {
  __TAURI_INTERNALS__?: unknown;
  __TAURI__?: unknown;
};

const tauriWindow = window as Window & TauriGlobals;
let savedInternals: unknown;
let savedTauri: unknown;

beforeAll(() => {
  savedInternals = tauriWindow.__TAURI_INTERNALS__;
  savedTauri = tauriWindow.__TAURI__;
  delete tauriWindow.__TAURI_INTERNALS__;
  delete tauriWindow.__TAURI__;
  installBrowserIpcFixtures();
});

afterAll(() => {
  clearIpcFixturesForTest();
  if (savedInternals !== undefined) {
    tauriWindow.__TAURI_INTERNALS__ = savedInternals;
  }
  if (savedTauri !== undefined) {
    tauriWindow.__TAURI__ = savedTauri;
  }
});

describe("browser fixtures drive real store loaders", () => {
  it("platformStore.initialize populates agents and counts", async () => {
    await usePlatformStore.getState().initialize();

    const state = usePlatformStore.getState();
    expect(state.agents.length).toBeGreaterThan(0);
    expect(state.skillsByAgent["claude-code"]).toBe(1);
    expect(state.error).toBeNull();
  });

  it("skillStore.getSkillsByAgent loads fixture skills", async () => {
    await useSkillStore.getState().getSkillsByAgent("claude-code");

    const state = useSkillStore.getState();
    expect(state.skillsByAgent["claude-code"]?.length).toBeGreaterThan(0);
    expect(state.error).toBeNull();
  });

  it("skillStore uninstall paths reject with the desktop-only string", async () => {
    await expect(
      useSkillStore
        .getState()
        .uninstallSkillFromAgent("fixture-central-skill", "claude-code"),
    ).rejects.toEqual(DESKTOP_ONLY_UNINSTALL_ERROR);
    expect(useSkillStore.getState().error).toBe(DESKTOP_ONLY_UNINSTALL_ERROR);

    await expect(
      useSkillStore
        .getState()
        .batchUninstallSkillsFromAgent("claude-code", [
          { skill_id: "fixture-central-skill" },
        ]),
    ).rejects.toEqual(DESKTOP_ONLY_UNINSTALL_ERROR);
  });

  it("usageStore.refresh populates overview/providers/scope", async () => {
    await useUsageStore.getState().refresh();

    const state = useUsageStore.getState();
    expect(state.overview).not.toBeNull();
    expect(state.providers.length).toBeGreaterThan(0);
    expect(state.scope?.targetId).toBe("local");
    expect(state.error).toBeNull();
  });

  it("targetStore.loadTargets keeps the local target", async () => {
    await useTargetStore.getState().loadTargets();

    const state = useTargetStore.getState();
    expect(state.targets.length).toBeGreaterThan(0);
    expect(state.activeTarget.id).toBe("local");
    expect(state.error).toBeNull();
  });

  it("operationLogStore.loadLogs pages fixture entries", async () => {
    await useOperationLogStore.getState().loadLogs();

    const state = useOperationLogStore.getState();
    expect(state.entries.length).toBeGreaterThan(0);
    expect(state.error).toBeNull();
  });

  it("runtimeLogStore.loadFiles lists fixture files", async () => {
    await useRuntimeLogStore.getState().loadFiles();

    const state = useRuntimeLogStore.getState();
    expect(state.files.length).toBeGreaterThan(0);
    expect(state.error).toBeNull();
  });

  it("tagGroupsStore CRUD round-trips through the stateful fixture", async () => {
    await useTagGroupsStore.getState().loadTagGroups();
    expect(useTagGroupsStore.getState().error).toBeNull();

    const created = await useTagGroupsStore
      .getState()
      .createTagGroup({ name: "演示分组" });
    expect(created.id).toMatch(/^fixture-tag-group-/);

    const state = useTagGroupsStore.getState();
    expect(state.groups.some((group) => group.id === created.id)).toBe(true);
    expect(state.error).toBeNull();
  });

  it("savedViewsStore CRUD round-trips through the stateful fixture", async () => {
    await useSavedViewsStore.getState().loadSavedViews();
    expect(useSavedViewsStore.getState().error).toBeNull();

    const created = await useSavedViewsStore
      .getState()
      .createSavedView({ name: "演示视图", query: "q=fixture" });
    expect(created.id).toMatch(/^fixture-saved-view-/);

    const state = useSavedViewsStore.getState();
    expect(state.views.some((view) => view.id === created.id)).toBe(true);
    expect(state.error).toBeNull();
  });
});
