import { fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useHotkey } from "@/hooks/useHotkey";
import { setNavigatorPlatform } from "@/test/support/testPlatform";

let restorePlatform: (() => void) | undefined;

function HotkeyProbe({ onTrigger }: { onTrigger: () => void }) {
  useHotkey("mod+k", onTrigger);
  return <div>probe</div>;
}

function setPlatform(platform: string) {
  restorePlatform?.();
  restorePlatform = setNavigatorPlatform(platform);
}

describe("useHotkey", () => {
  afterEach(() => {
    restorePlatform?.();
    restorePlatform = undefined;
  });

  it("fires mod+k from Ctrl+K on non-macOS", () => {
    setPlatform("Win32");
    const onTrigger = vi.fn();
    render(<HotkeyProbe onTrigger={onTrigger} />);

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });

    expect(onTrigger).toHaveBeenCalledTimes(1);
  });

  it("fires mod+k from Command+K on macOS", () => {
    setPlatform("MacIntel");
    const onTrigger = vi.fn();
    render(<HotkeyProbe onTrigger={onTrigger} />);

    fireEvent.keyDown(window, { key: "k", metaKey: true });

    expect(onTrigger).toHaveBeenCalledTimes(1);
  });

  it("ignores mod+k with extra modifiers", () => {
    setPlatform("Win32");
    const onTrigger = vi.fn();
    render(<HotkeyProbe onTrigger={onTrigger} />);

    fireEvent.keyDown(window, { key: "k", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(window, { key: "k", ctrlKey: true, altKey: true });

    expect(onTrigger).not.toHaveBeenCalled();
  });
});
