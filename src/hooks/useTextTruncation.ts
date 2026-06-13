import { useEffect, useRef, useState } from "react";

export function useTextTruncation<T extends HTMLElement = HTMLElement>(
  text: string | undefined | null,
) {
  const ref = useRef<T | null>(null);
  const [isTruncated, setIsTruncated] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (!node) {
      setIsTruncated(false);
      return;
    }

    const measure = () => {
      setIsTruncated(node.scrollHeight - node.clientHeight > 1);
    };

    measure();

    if (typeof ResizeObserver === "undefined") {
      return;
    }
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, [text]);

  return { ref, isTruncated };
}
