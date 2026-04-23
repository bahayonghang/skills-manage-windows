export function markAppPerformance(name: string) {
  if (typeof window === "undefined" || typeof window.performance?.mark !== "function") {
    return;
  }

  window.performance.mark(name);
}
