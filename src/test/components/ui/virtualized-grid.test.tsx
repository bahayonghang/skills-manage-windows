import { createRef, type RefObject } from "react";
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { VirtualizedGrid } from "@/components/ui/virtualized-grid";

function renderGrid(itemCount: number, itemHeight = 200) {
  const items = Array.from({ length: itemCount }, (_, index) => index);
  const scrollContainerRef = createRef<HTMLDivElement>() as RefObject<HTMLDivElement | null>;
  const utils = render(
    <div ref={scrollContainerRef}>
      <VirtualizedGrid
        items={items}
        itemHeight={itemHeight}
        rowGap={16}
        columnGap={16}
        overscanRows={50}
        minColumnWidth={100}
        maxColumns={2}
        scrollContainerRef={scrollContainerRef}
        itemKey={(item) => item}
        renderItem={(item) => <div data-testid={`cell-${item}`}>item {item}</div>}
      />
    </div>,
  );
  return utils;
}

describe("VirtualizedGrid row height contract", () => {
  it("pins each row track to the fixed itemHeight so cards cannot spill into the next row", () => {
    const { container } = renderGrid(4, 200);
    const rows = container.querySelectorAll('[role="row"]');
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) {
      const el = row as HTMLElement;
      expect(el.style.height).toBe("200px");
      // 隐式行轨道必须钉为 100%，否则 grid auto 行会按卡片内容撑高，
      // 导致卡片压到下一行（Central 网格重叠 bug 的回归防线）。
      expect(el.style.gridTemplateRows).toBe("100%");
    }
  });

  it("keeps row offsets on the fixed stride", () => {
    const { container } = renderGrid(4, 200);
    const rows = Array.from(container.querySelectorAll('[role="row"]')) as HTMLElement[];
    const tops = rows.map((row) => Number.parseInt(row.style.top, 10));
    const sorted = [...tops].sort((a, b) => a - b);
    for (let i = 1; i < sorted.length; i += 1) {
      expect(sorted[i] - sorted[i - 1]).toBe(216);
    }
  });
});
