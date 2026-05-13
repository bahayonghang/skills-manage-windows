import { useResizableWidth } from "@/hooks/useResizableWidth";

const CENTRAL_FILTER_DEFAULT_WIDTH = 286;
const CENTRAL_FILTER_MIN_WIDTH = 220;
const CENTRAL_FILTER_MAX_WIDTH = 460;
const CENTRAL_CATEGORIZE_DEFAULT_WIDTH = 392;
const CENTRAL_CATEGORIZE_MIN_WIDTH = 336;
const CENTRAL_CATEGORIZE_MAX_WIDTH = 640;

export function useCentralSkillsLayoutSizing() {
  const filterSidebar = useResizableWidth({
    defaultWidth: CENTRAL_FILTER_DEFAULT_WIDTH,
    minWidth: CENTRAL_FILTER_MIN_WIDTH,
    maxWidth: CENTRAL_FILTER_MAX_WIDTH,
  });
  const categorizeSidebar = useResizableWidth({
    defaultWidth: CENTRAL_CATEGORIZE_DEFAULT_WIDTH,
    minWidth: CENTRAL_CATEGORIZE_MIN_WIDTH,
    maxWidth: CENTRAL_CATEGORIZE_MAX_WIDTH,
    resizeFrom: "left",
  });

  return {
    filterSidebarWidth: filterSidebar.width,
    startFilterSidebarResize: filterSidebar.startResize,
    handleFilterSidebarResizeKeyDown: filterSidebar.handleResizeKeyDown,
    categorizeSidebarWidth: categorizeSidebar.width,
    startCategorizeSidebarResize: categorizeSidebar.startResize,
    handleCategorizeSidebarResizeKeyDown: categorizeSidebar.handleResizeKeyDown,
  };
}
