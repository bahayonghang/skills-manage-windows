import {
  useEffect,
  useMemo,
  useState,
  type Key,
  type ReactNode,
  type RefObject,
} from "react";

interface VirtualizedGridProps<T> {
  items: T[];
  itemHeight: number;
  rowGap?: number;
  columnGap?: number;
  overscanRows?: number;
  fallbackHeight?: number;
  fallbackWidth?: number;
  minColumnWidth?: number;
  maxColumns?: number;
  scrollContainerRef: RefObject<HTMLElement | null>;
  itemKey: (item: T, index: number) => Key;
  renderItem: (item: T, index: number) => ReactNode;
}

export function VirtualizedGrid<T>({
  items,
  itemHeight,
  rowGap = 16,
  columnGap = 16,
  overscanRows = 4,
  fallbackHeight = 640,
  fallbackWidth = 960,
  minColumnWidth = 420,
  maxColumns = 2,
  scrollContainerRef,
  itemKey,
  renderItem,
}: VirtualizedGridProps<T>) {
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(fallbackHeight);
  const [viewportWidth, setViewportWidth] = useState(fallbackWidth);
  const rowStride = itemHeight + rowGap;

  useEffect(() => {
    const scrollNode = scrollContainerRef.current;
    if (!scrollNode) return;

    const updateViewport = () => {
      setViewportHeight(scrollNode.clientHeight > 0 ? scrollNode.clientHeight : fallbackHeight);
      setViewportWidth(scrollNode.clientWidth > 0 ? scrollNode.clientWidth : fallbackWidth);
    };

    const updateScroll = () => {
      setScrollTop(scrollNode.scrollTop);
    };

    updateViewport();
    updateScroll();

    scrollNode.addEventListener("scroll", updateScroll, { passive: true });

    if (typeof ResizeObserver !== "undefined") {
      const resizeObserver = new ResizeObserver(updateViewport);
      resizeObserver.observe(scrollNode);
      return () => {
        resizeObserver.disconnect();
        scrollNode.removeEventListener("scroll", updateScroll);
      };
    }

    window.addEventListener("resize", updateViewport);
    return () => {
      window.removeEventListener("resize", updateViewport);
      scrollNode.removeEventListener("scroll", updateScroll);
    };
  }, [fallbackHeight, fallbackWidth, scrollContainerRef]);

  const columns = useMemo(() => {
    const calculated = Math.max(1, Math.floor((viewportWidth + columnGap) / (minColumnWidth + columnGap)));
    return Math.min(maxColumns, calculated);
  }, [columnGap, maxColumns, minColumnWidth, viewportWidth]);

  const rowCount = useMemo(
    () => (items.length === 0 ? 0 : Math.ceil(items.length / columns)),
    [columns, items.length]
  );

  const visibleRange = useMemo(() => {
    if (rowCount === 0) {
      return { start: 0, end: 0 };
    }

    const visibleRows = Math.max(1, Math.ceil(viewportHeight / rowStride));
    const maxStart = Math.max(0, rowCount - visibleRows);
    const start = Math.min(
      maxStart,
      Math.max(0, Math.floor(scrollTop / rowStride) - overscanRows)
    );
    const end = Math.min(rowCount, start + visibleRows + overscanRows * 2);

    return { start, end };
  }, [overscanRows, rowCount, rowStride, scrollTop, viewportHeight]);

  const totalHeight = rowCount > 0 ? rowCount * rowStride - rowGap : 0;

  return (
    <div role="grid" className="relative w-full" style={{ height: totalHeight }}>
      {Array.from(
        { length: Math.max(0, visibleRange.end - visibleRange.start) },
        (_, offset) => visibleRange.start + offset
      ).map((rowIndex) => {
        const rowItems = items.slice(rowIndex * columns, rowIndex * columns + columns);

        return (
          <div
            key={`row-${rowIndex}`}
            role="row"
            className="absolute left-0 right-0 grid"
            style={{
              top: rowIndex * rowStride,
              height: itemHeight,
              columnGap,
              gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
            }}
          >
            {rowItems.map((item, columnIndex) => {
              const index = rowIndex * columns + columnIndex;
              return (
                <div key={itemKey(item, index)} role="gridcell" className="min-w-0">
                  {renderItem(item, index)}
                </div>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}
