import { RefObject, useCallback, useEffect } from "react";

interface UseLogsKeyboardOptions {
  searchInputRef: RefObject<HTMLInputElement | null>;
  rowsContainerRef: RefObject<HTMLElement | null>;
  onRefresh: () => void;
  onExport: () => void;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (target.isContentEditable) return true;
  return false;
}

export function useLogsKeyboard({
  searchInputRef,
  rowsContainerRef,
  onRefresh,
  onExport,
}: UseLogsKeyboardOptions) {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      const editing = isEditableTarget(event.target);

      if (event.key === "/" && !editing) {
        event.preventDefault();
        searchInputRef.current?.focus();
        return;
      }
      if (!editing && (event.key === "r" || event.key === "R")) {
        event.preventDefault();
        onRefresh();
        return;
      }
      if (!editing && (event.key === "e" || event.key === "E")) {
        event.preventDefault();
        onExport();
        return;
      }
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        const container = rowsContainerRef.current;
        if (!container) return;
        const rows = Array.from(
          container.querySelectorAll<HTMLButtonElement>("button[data-logs-row]"),
        );
        if (rows.length === 0) return;
        const currentIndex = rows.findIndex((row) => row === event.target);
        if (currentIndex === -1 && editing) return;
        const offset = event.key === "ArrowDown" ? 1 : -1;
        const nextIndex =
          currentIndex === -1
            ? offset === 1
              ? 0
              : rows.length - 1
            : Math.min(rows.length - 1, Math.max(0, currentIndex + offset));
        if (nextIndex !== currentIndex || currentIndex === -1) {
          event.preventDefault();
          rows[nextIndex]?.focus();
        }
      }
    },
    [searchInputRef, rowsContainerRef, onRefresh, onExport],
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleKeyDown]);
}
