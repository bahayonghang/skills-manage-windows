import { describe, expect, it } from "vitest";
import { getRepoDotColor, getTagColor } from "@/lib/tagColor";

describe("getTagColor", () => {
  it("hex 颜色优先于哈希", () => {
    const a = getTagColor({ id: "1", name: "anything", color: "#ff8800" });
    expect(a.style?.color).toBeTruthy();
    expect(a.className).toBeUndefined();
  });

  it("无颜色时按名哈希且确定", () => {
    const a = getTagColor({ id: "1", name: "frontend" });
    const b = getTagColor({ id: "2", name: "frontend" });
    expect(a.className).toBe(b.className);
    expect(a.className).toBeTruthy();
  });

  it("不同名分散到不同色（大概率）", () => {
    const names = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const classes = new Set(
      names.map((n) => getTagColor({ id: n, name: n }).className),
    );
    expect(classes.size).toBeGreaterThan(1);
  });
});

describe("getRepoDotColor", () => {
  it("同名确定性返回同色", () => {
    expect(getRepoDotColor("anthropics/skills")).toBe(
      getRepoDotColor("anthropics/skills"),
    );
  });

  it("返回非空颜色串", () => {
    expect(getRepoDotColor("anthropics/skills")).toBeTruthy();
  });
});
