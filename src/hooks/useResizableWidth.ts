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
}

function clampWidth(width: number, minWidth: number, maxWidth: number) {
  return Math.min(Math.max(width, minWidth), maxWidth);
}

export function useResizableWidth({
  defaultWidth,
  minWidth,
  maxWidth,
  step = 16,
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
        setWidth(
          clampWidth(startWidth + moveEvent.clientX - startX, minWidth, maxWidth)
        );
      }

      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", finishResize);
      window.addEventListener("pointercancel", finishResize);
      event.preventDefault();
    },
    [maxWidth, minWidth, width]
  );

  const handleResizeKeyDown = useCallback(
    (event: KeyboardEvent<HTMLElement>) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        resizeBy(-step);
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        resizeBy(step);
      }
    },
    [resizeBy, step]
  );

  return {
    width,
    resizeBy,
    startResize,
    handleResizeKeyDown,
  };
}
