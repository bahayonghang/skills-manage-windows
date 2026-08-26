import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { SkillsCliHeader } from "@/components/skillsCli/SkillsCliHeader";

const counts = {
  installed: 4,
  linked: 2,
  unlinked: 1,
  repositories: 3,
};

describe("SkillsCliHeader", () => {
  it("renders four counts and a successful runtime status", () => {
    render(
      <SkillsCliHeader
        counts={counts}
        doctor={{ nodeVersion: "v22.20.0", npmSpec: "skills@1.5.23" }}
        runtimeError={null}
        isLoading={false}
        isRefreshing={false}
        installAvailable
        onRefresh={vi.fn()}
        onCheckUpdates={vi.fn()}
        onOpenInstall={vi.fn()}
      />,
    );
    const strip = screen.getByTestId("skills-cli-counts");
    expect(strip).toHaveTextContent("4");
    expect(strip).toHaveTextContent("2");
    expect(strip).toHaveTextContent("1");
    expect(strip).toHaveTextContent("3");
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "skills@1.5.23",
    );
    expect(screen.getByRole("button", { name: "安装技能" })).toBeEnabled();
  });

  it("shows a safe runtime error, disables Install, and keeps counts", () => {
    render(
      <SkillsCliHeader
        counts={counts}
        doctor={null}
        runtimeError="skills_cli.cli_unavailable:The Skills CLI package could not be executed."
        isLoading={false}
        isRefreshing={false}
        installAvailable
        onRefresh={vi.fn()}
        onCheckUpdates={vi.fn()}
        onOpenInstall={vi.fn()}
      />,
    );
    expect(screen.getByTestId("skills-cli-doctor")).toHaveTextContent(
      "无法执行 Skills CLI 软件包。",
    );
    expect(screen.getByRole("button", { name: "安装技能" })).toBeDisabled();
    expect(screen.getByTestId("skills-cli-counts")).toHaveTextContent("4");
  });

  it("disables Install when the mount is unavailable and Refresh while refreshing", () => {
    const onRefresh = vi.fn();
    const onOpenInstall = vi.fn();
    render(
      <SkillsCliHeader
        counts={counts}
        doctor={{ nodeVersion: "v22.20.0", npmSpec: "skills@1.5.23" }}
        runtimeError={null}
        isLoading={false}
        isRefreshing
        installAvailable={false}
        onRefresh={onRefresh}
        onCheckUpdates={vi.fn()}
        onOpenInstall={onOpenInstall}
      />,
    );
    expect(screen.getByRole("button", { name: "刷新" })).toBeDisabled();
    const install = screen.getByRole("button", { name: "安装技能" });
    expect(install).toBeDisabled();
    fireEvent.click(install);
    expect(onOpenInstall).not.toHaveBeenCalled();
  });

  it("keeps Check updates enabled when the runtime is blocked", () => {
    const onCheckUpdates = vi.fn();
    render(
      <SkillsCliHeader
        counts={counts}
        doctor={null}
        runtimeError="skills_cli.cli_unavailable:The Skills CLI package could not be executed."
        isLoading={false}
        isRefreshing={false}
        installAvailable
        onRefresh={vi.fn()}
        onCheckUpdates={onCheckUpdates}
        onOpenInstall={vi.fn()}
      />,
    );
    const check = screen.getByRole("button", { name: "检查更新" });
    expect(check).toBeEnabled();
    fireEvent.click(check);
    expect(onCheckUpdates).toHaveBeenCalledTimes(1);
  });
});
