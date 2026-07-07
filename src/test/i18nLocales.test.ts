import i18next from "i18next";
import { describe, expect, it } from "vitest";

import en from "@/i18n/locales/en.json";
import zh from "@/i18n/locales/zh.json";

async function createI18n(language: "en" | "zh") {
  const i18n = i18next.createInstance();
  await i18n.init({
    lng: language,
    fallbackLng: language,
    resources: {
      en: { translation: en },
      zh: { translation: zh },
    },
    interpolation: {
      escapeValue: false,
    },
  });
  return i18n;
}

describe("i18n locale strings", () => {
  it("renders English count labels without ICU leftovers", async () => {
    const i18n = await createI18n("en");
    const labels = [
      i18n.t("installDialog.confirmInstall", { count: 4 }),
      i18n.t("batchInstall.install", { count: 4 }),
      i18n.t("detail.frontmatterItems", { count: 2 }),
      i18n.t("skillPicker.addCount", { count: 2 }),
    ];

    expect(labels).toEqual([
      "Install to 4 platform(s)",
      "Install to 4 platform(s)",
      "2 item(s)",
      "Add 2 Skill(s)",
    ]);
    expect(labels.join(" ")).not.toMatch(/[{}]/);
  });
});
