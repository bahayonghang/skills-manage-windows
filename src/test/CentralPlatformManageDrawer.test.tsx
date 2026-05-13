import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import drawerSource from "../components/central/CentralPlatformManageDrawer.tsx?raw";
import { CentralPlatformManageDrawer } from "../components/central/CentralPlatformManageDrawer";
import type { AgentWithStatus } from "../types";

type PlatformVisibilitySectionMockProps = {
  onToggleCategory: (category: "coding" | "lobster", visible: boolean) => void;
  onTogglePlatform: (agentId: string, enabled: boolean) => void;
};

type CustomPlatformsSectionMockProps = {
  onAddPlatform: () => void;
  onEditPlatform: (agent: AgentWithStatus) => void;
  onRemovePlatform: (agentId: string) => void;
};

type PlatformDialogMockProps = {
  open: boolean;
  platform: AgentWithStatus | null;
  onAdd: (displayName: string, globalSkillsDir: string, category?: string) => Promise<void>;
  onEdit: (displayName: string, globalSkillsDir: string, category?: string) => Promise<void>;
};

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock("@/components/settings/PlatformVisibilitySettingsSection", () => ({
  PlatformVisibilitySettingsSection: ({
    onToggleCategory,
    onTogglePlatform,
  }: PlatformVisibilitySectionMockProps) => (
    <div>
      <button type="button" onClick={() => onToggleCategory("coding", false)}>
        toggle-category-coding
      </button>
      <button type="button" onClick={() => onTogglePlatform("custom-1", false)}>
        toggle-platform-custom-1
      </button>
    </div>
  ),
}));

vi.mock("@/components/settings/CustomPlatformsSettingsSection", async () => {
  const React = await vi.importActual<typeof import("react")>("react");

  return {
    CustomPlatformsSettingsSection: ({
      onAddPlatform,
      onEditPlatform,
      onRemovePlatform,
    }: CustomPlatformsSectionMockProps) => {
      const [confirmingDelete, setConfirmingDelete] = React.useState(false);

      return (
        <div>
          <button type="button" onClick={onAddPlatform}>
            添加自定义平台
          </button>
          <button
            type="button"
            onClick={() =>
              onEditPlatform({
                id: "custom-1",
                display_name: "Custom Platform",
                global_skills_dir: "C:/skills/custom",
                category: "coding",
                is_builtin: false,
                is_detected: true,
                is_enabled: true,
              })
            }
          >
            编辑平台 Custom Platform
          </button>
          <button type="button" onClick={() => setConfirmingDelete(true)}>
            删除平台
          </button>
          {confirmingDelete ? (
            <button type="button" onClick={() => onRemovePlatform("custom-1")}>
              删除
            </button>
          ) : null}
        </div>
      );
    },
  };
});

vi.mock("@/components/settings/PlatformDialog", async () => {
  const React = await vi.importActual<typeof import("react")>("react");

  return {
    PlatformDialog: ({ open, platform, onAdd, onEdit }: PlatformDialogMockProps) => {
      const [name, setName] = React.useState(platform?.display_name ?? "");
      const [dir, setDir] = React.useState(platform?.global_skills_dir ?? "");

      React.useEffect(() => {
        if (open) {
          setName(platform?.display_name ?? "");
          setDir(platform?.global_skills_dir ?? "");
        }
      }, [open, platform]);

      if (!open) return null;

      return (
        <div>
          <label>
            平台名称
            <input
              aria-label="平台名称"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </label>
          <label>
            技能目录路径
            <input
              aria-label="技能目录路径"
              value={dir}
              onChange={(event) => setDir(event.target.value)}
            />
          </label>
          {platform ? (
            <button type="button" onClick={() => onEdit(name, dir, "coding")}>
              保存
            </button>
          ) : (
            <button type="button" onClick={() => onAdd(name, dir, "coding")}>
              添加
            </button>
          )}
        </div>
      );
    },
  };
});

vi.mock("@/pages/settingsViewModel", () => ({
  getNormalizedPlatformVisibilityQuery: vi.fn(() => ""),
  getPlatformVisibilityGroups: vi.fn(() => []),
}));

vi.mock("@/pages/settingsViewActions", () => ({
  createSettingsViewActions: vi.fn(),
}));

const agents: AgentWithStatus[] = [
  {
    id: "custom-1",
    display_name: "Custom Platform",
    global_skills_dir: "C:/skills/custom",
    category: "coding",
    is_builtin: false,
    is_detected: true,
    is_enabled: true,
  },
];

describe("CentralPlatformManageDrawer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("adds, edits, removes platforms and toggles visibility", async () => {
    const addCustomAgent = vi.fn().mockResolvedValue(undefined);
    const updateCustomAgent = vi.fn().mockResolvedValue(undefined);
    const removeCustomAgent = vi.fn().mockResolvedValue(undefined);
    const setCategoryVisibility = vi.fn().mockResolvedValue(undefined);
    const setAgentEnabled = vi.fn().mockResolvedValue(undefined);
    const refreshAfterPlatformChange = vi.fn().mockResolvedValue(undefined);

    render(
      <CentralPlatformManageDrawer
        open
        onOpenChange={vi.fn()}
        agents={agents}
        categoryVisibility={{ coding: true, lobster: true }}
        addCustomAgent={addCustomAgent}
        updateCustomAgent={updateCustomAgent}
        removeCustomAgent={removeCustomAgent}
        setCategoryVisibility={setCategoryVisibility}
        setAgentEnabled={setAgentEnabled}
        refreshAfterPlatformChange={refreshAfterPlatformChange}
      />
    );

    fireEvent.click(screen.getByText("添加自定义平台"));
    fireEvent.change(screen.getByLabelText("平台名称"), { target: { value: "New Platform" } });
    fireEvent.change(screen.getByLabelText("技能目录路径"), { target: { value: "C:/skills/new" } });
    fireEvent.click(screen.getByText("添加"));

    await waitFor(() => {
      expect(addCustomAgent).toHaveBeenCalledWith({
        display_name: "New Platform",
        global_skills_dir: "C:/skills/new",
        category: "coding",
      });
      expect(refreshAfterPlatformChange).toHaveBeenCalled();
    });

    fireEvent.click(screen.getByText("编辑平台 Custom Platform"));
    fireEvent.change(screen.getByLabelText("平台名称"), { target: { value: "Custom Platform v2" } });
    fireEvent.change(screen.getByLabelText("技能目录路径"), { target: { value: "C:/skills/custom-v2" } });
    fireEvent.click(screen.getByText("保存"));

    await waitFor(() => {
      expect(updateCustomAgent).toHaveBeenCalledWith("custom-1", {
        display_name: "Custom Platform v2",
        global_skills_dir: "C:/skills/custom-v2",
        category: "coding",
      });
      expect(refreshAfterPlatformChange).toHaveBeenCalledTimes(2);
    });

    fireEvent.click(screen.getByText("删除平台"));
    fireEvent.click(screen.getByText("删除"));

    await waitFor(() => {
      expect(removeCustomAgent).toHaveBeenCalledWith("custom-1");
      expect(refreshAfterPlatformChange).toHaveBeenCalledTimes(3);
    });

    fireEvent.click(screen.getByText("toggle-category-coding"));
    fireEvent.click(screen.getByText("toggle-platform-custom-1"));

    await waitFor(() => {
      expect(setCategoryVisibility).toHaveBeenCalledWith("coding", false);
      expect(setAgentEnabled).toHaveBeenCalledWith("custom-1", false);
    });
  });

  it("no longer depends on settingsViewActions or unsupported placeholders", () => {
    expect(drawerSource).not.toContain("@/pages/settingsViewActions");
    expect(drawerSource).not.toContain("unsupported");
    expect(drawerSource).not.toContain("noopAsync");
  });
});
