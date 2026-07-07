import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { ShortcutsSheet } from "@/components/layout/ShortcutsSheet";
import { setNavigatorPlatform } from "./testPlatform";

let restorePlatform: (() => void) | undefined;

function setPlatform(platform: string) {
  restorePlatform?.();
  restorePlatform = setNavigatorPlatform(platform);
}

function Harness({ initialOpen = false }: { initialOpen?: boolean }) {
  const [open, setOpen] = useState(initialOpen);
  return (
    <div>
      <input aria-label="probe-input" />
      <ShortcutsSheet open={open} onOpenChange={setOpen} />
    </div>
  );
}

function queryTitle() {
  return screen.queryByText("键盘快捷键");
}

describe("ShortcutsSheet", () => {
  beforeEach(() => {
    setPlatform("Win32");
  });

  afterEach(() => {
    restorePlatform?.();
    restorePlatform = undefined;
  });

  it('opens on "?" outside editable focus and closes with Escape', async () => {
    render(<Harness />);
    expect(queryTitle()).not.toBeInTheDocument();

    fireEvent.keyDown(window, { key: "?", shiftKey: true });

    const title = await screen.findByText("键盘快捷键");
    expect(title).toBeInTheDocument();

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    await waitFor(() => {
      expect(queryTitle()).not.toBeInTheDocument();
    });
  });

  it('does not open on "?" while an input is focused', () => {
    render(<Harness />);
    const input = screen.getByLabelText("probe-input");
    input.focus();

    fireEvent.keyDown(input, { key: "?", shiftKey: true });

    expect(queryTitle()).not.toBeInTheDocument();
  });

  it("opens on mod+/ (Ctrl+/ on non-mac)", async () => {
    render(<Harness />);

    fireEvent.keyDown(window, { key: "/", ctrlKey: true });

    expect(await screen.findByText("键盘快捷键")).toBeInTheDocument();
  });

  it("renders every registry group and the key entries", async () => {
    render(<Harness initialOpen />);
    await screen.findByText("键盘快捷键");

    // Group titles
    expect(screen.getByText("全局")).toBeInTheDocument();
    expect(screen.getByText("中央技能库")).toBeInTheDocument();
    expect(screen.getByText("操作日志")).toBeInTheDocument();
    expect(screen.getByText("目标快切")).toBeInTheDocument();

    // Key entries: mod+k (global search + command palette), Logs "/", targets Esc
    expect(screen.getByText("打开/关闭全局搜索")).toBeInTheDocument();
    expect(screen.getByText("打开/关闭命令面板")).toBeInTheDocument();
    expect(screen.getAllByLabelText("Ctrl+K")).toHaveLength(2);
    expect(screen.getByText("聚焦日志搜索框")).toBeInTheDocument();
    expect(screen.getByLabelText("/")).toBeInTheDocument();
    expect(screen.getByText("关闭目标切换菜单")).toBeInTheDocument();
    expect(screen.getByLabelText("Escape")).toBeInTheDocument();
  });
});
