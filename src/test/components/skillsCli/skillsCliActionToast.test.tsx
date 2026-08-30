import { describe, expect, it, vi } from "vitest";

import {
  SKILLS_CLI_ACTION_TOAST_DURATION_MS,
  SKILLS_CLI_ACTION_TOAST_ID,
  SKILLS_CLI_TOAST_ICONS,
  showSkillsCliActionToast,
} from "@/components/skillsCli/skillsCliActionToast";
import { statusTextClass } from "@/lib/statusTone";

vi.mock("sonner", () => ({
  toast: vi.fn(),
}));

import { toast } from "sonner";

const toastMock = vi.mocked(toast);

describe("showSkillsCliActionToast", () => {
  it("uses a stable id, 2800ms duration, and replaces by that id", () => {
    showSkillsCliActionToast({ semantic: "success", message: "first" });
    showSkillsCliActionToast({ semantic: "error", message: "second" });
    expect(toastMock).toHaveBeenCalledTimes(2);
    expect(toastMock).toHaveBeenNthCalledWith(
      1,
      "first",
      expect.objectContaining({
        id: SKILLS_CLI_ACTION_TOAST_ID,
        duration: SKILLS_CLI_ACTION_TOAST_DURATION_MS,
      }),
    );
    expect(toastMock).toHaveBeenNthCalledWith(
      2,
      "second",
      expect.objectContaining({
        id: SKILLS_CLI_ACTION_TOAST_ID,
        duration: 2_800,
      }),
    );
    expect(SKILLS_CLI_ACTION_TOAST_ID).toBe("skills-cli-action");
  });

  it("maps the four reviewed icon and tone pairs", () => {
    const cases = [
      ["success", SKILLS_CLI_TOAST_ICONS.success, statusTextClass.success],
      ["error", SKILLS_CLI_TOAST_ICONS.error, statusTextClass.error],
      [
        "destructiveSuccess",
        SKILLS_CLI_TOAST_ICONS.destructiveSuccess,
        statusTextClass.success,
      ],
      [
        "destructiveError",
        SKILLS_CLI_TOAST_ICONS.destructiveError,
        statusTextClass.error,
      ],
    ] as const;

    for (const [semantic, icon, toneClass] of cases) {
      toastMock.mockClear();
      showSkillsCliActionToast({ semantic, message: semantic });
      const options = toastMock.mock.calls[0]?.[1] as {
        icon: { type: unknown; props: { className: string } };
      };
      expect(options.icon.type).toBe(icon);
      expect(options.icon.props.className).toContain(toneClass);
    }
  });

  it("accepts only a localized string message", () => {
    showSkillsCliActionToast({ semantic: "success", message: "已安装" });
    expect(toastMock).toHaveBeenCalledWith("已安装", expect.any(Object));
    showSkillsCliActionToast({
      semantic: "error",
      // @ts-expect-error helper 不接受 raw backend object
      message: { code: "skills_cli.busy" },
    });
  });
});
