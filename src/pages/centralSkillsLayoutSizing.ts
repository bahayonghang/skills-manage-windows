import { useResizableWidth } from "@/hooks/useResizableWidth";

const CENTRAL_FILTER_DEFAULT_WIDTH = 286;
const CENTRAL_FILTER_MIN_WIDTH = 220;
const CENTRAL_FILTER_MAX_WIDTH = 460;

export function useCentralSkillsLayoutSizing() {
  const filterSidebar = useResizableWidth({
    defaultWidth: CENTRAL_FILTER_DEFAULT_WIDTH,
    minWidth: CENTRAL_FILTER_MIN_WIDTH,
    maxWidth: CENTRAL_FILTER_MAX_WIDTH,
  });

  return {
    filterSidebarWidth: filterSidebar.width,
    startFilterSidebarResize: filterSidebar.startResize,
    handleFilterSidebarResizeKeyDown: filterSidebar.handleResizeKeyDown,
  };
}
