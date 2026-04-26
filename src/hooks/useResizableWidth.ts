import {
  useCallback,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";

interface ResizableWidthOptions {
  defaultWidth: number;
  minWidth: number;
  maxWidth: number;
  step?: number;
  resizeFrom?: "left" | "right";
}

function clampWidth(width: number, minWidth: number, maxWidth: number) {
  return Math.min(Math.max(width, minWidth), maxWidth);
}

export function useResizableWidth({
  defaultWidth,
  minWidth,
  maxWidth,
  step = 16,
  resizeFrom = "right",
}: ResizableWidthOptions) {
  const [width, setWidth] = useState(() =>
    clampWidth(defaultWidth, minWidth, maxWidth)
  );

  const resizeBy = useCallback(
    (delta: number) => {
      setWidth((current) => clampWidth(current + delta, minWidth, maxWidth));
    },
    [maxWidth, minWidth]
  );

  const startResize = useCallback(
    (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return;

      const startX = event.clientX;
      const startWidth = width;
      const previousCursor = document.body.style.cursor;
      const previousUserSelect = document.body.style.userSelect;

      function finishResize() {
        window.removeEventListener("pointermove", handlePointerMove);
        window.removeEventListener("pointerup", finishResize);
        window.removeEventListener("pointercancel", finishResize);
        document.body.style.cursor = previousCursor;
        document.body.style.userSelect = previousUserSelect;
      }

      function handlePointerMove(moveEvent: globalThis.PointerEvent) {
        const delta =
          resizeFrom === "left"
            ? startX - moveEvent.clientX
            : moveEvent.clientX - startX;
        setWidth(clampWidth(startWidth + delta, minWidth, maxWidth));
      }

      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", finishResize);
      window.addEventListener("pointercancel", finishResize);
      event.preventDefault();
    },
    [maxWidth, minWidth, resizeFrom, width]
  );

  const handleResizeKeyDown = useCallback(
    (event: KeyboardEvent<HTMLElement>) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        resizeBy(resizeFrom === "left" ? step : -step);
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        resizeBy(resizeFrom === "left" ? -step : step);
      }
    },
    [resizeBy, resizeFrom, step]
  );

  return {
    width,
    resizeBy,
    startResize,
    handleResizeKeyDown,
  };
}
