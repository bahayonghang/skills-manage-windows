import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";

import { SkillsCliCleanupDialog } from "@/components/skillsCli/SkillsCliCleanupDialog";
import type { CleanupCandidate } from "@/pages/skillsCliBatchModel";

const ASYNC_UI_TIMEOUT_MS = 5_000;

const candidates: CleanupCandidate[] = [
  {
    name: "ghost",
    group: "stale",
    reasons: [
      { platform: "Cursor", reasonCode: "canonical_missing" },
      { platform: "Amp", reasonCode: "canonical_missing" },
    ],
  },
  {
    name: "healthy",
    group: "platformUnavailable",
    reasons: [
      { platform: "Cursor", reasonCode: "platform_not_detected" },
      { platform: "Amp", reasonCode: "platform_disabled" },
    ],
  },
];

describe("SkillsCliCleanupDialog", () => {
  it("defaults stale entries on and platform-unavailable entries off", async () => {
    const onConfirm = vi.fn();
    render(
      <SkillsCliCleanupDialog
        open
        candidates={candidates}
        busy={false}
        onOpenChange={vi.fn()}
        onConfirm={onConfirm}
      />,
    );
    const dialog = await screen.findByRole(
      "dialog",
      { name: /清理 Unavailable/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(dialog);
    expect(surface.getByLabelText("选择 ghost 进行清理")).toBeChecked();
    expect(surface.getByLabelText("选择 healthy 进行清理")).not.toBeChecked();
    expect(surface.getByTestId("skills-cli-cleanup-group-stale")).toHaveTextContent(
      "1 个失效",
    );
    expect(
      surface.getByTestId("skills-cli-cleanup-group-platformUnavailable"),
    ).toHaveTextContent("1 个平台不可用");
    expect(
      surface.queryByTestId("skills-cli-cleanup-platform-risk"),
    ).not.toBeInTheDocument();
    expect(surface.getByText(/Cursor：规范目录已缺失/)).toBeInTheDocument();
    expect(surface.getByText(/Amp：该平台已被禁用/)).toBeInTheDocument();
    fireEvent.click(surface.getByRole("button", { name: "查看卸载影响" }));
    expect(onConfirm).toHaveBeenCalledWith(["ghost"]);
  });

  it("shows the platform risk copy only after a platform-unavailable skill is checked", async () => {
    render(
      <SkillsCliCleanupDialog
        open
        candidates={candidates}
        busy={false}
        onOpenChange={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );
    const dialog = await screen.findByRole(
      "dialog",
      { name: /清理 Unavailable/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(dialog);
    expect(
      surface.queryByTestId("skills-cli-cleanup-platform-risk"),
    ).not.toBeInTheDocument();
    fireEvent.click(surface.getByLabelText("选择 healthy 进行清理"));
    expect(surface.getByTestId("skills-cli-cleanup-platform-risk")).toHaveTextContent(
      "这些技能本体仍然健康",
    );
    fireEvent.click(surface.getByLabelText("选择 healthy 进行清理"));
    expect(
      surface.queryByTestId("skills-cli-cleanup-platform-risk"),
    ).not.toBeInTheDocument();
  });

  it("does not register a page-level keydown listener", () => {
    const source = readFileSync(
      resolve(process.cwd(), "src/components/skillsCli/SkillsCliCleanupDialog.tsx"),
      "utf8",
    );
    expect(source).not.toMatch(/addEventListener/);
    expect(source).not.toMatch(/from ["']@\/lib\/ipc["']/);
    expect(source).not.toMatch(/\binvoke\s*\(/);
  });
});
