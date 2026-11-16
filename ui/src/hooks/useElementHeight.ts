import { useLayoutEffect, useRef, useState } from 'react';

/// Measures an element's content height and re-measures on resize. The height
/// stays null until a measurement exists, and in environments without
/// ResizeObserver, so callers can treat unknown as "no limit".
export function useElementHeight<T extends HTMLElement>() {
  const ref = useRef<T | null>(null);
  const [height, setHeight] = useState<number | null>(null);

  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null || typeof ResizeObserver === 'undefined') {
      return;
    }

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry !== undefined) {
        setHeight(entry.contentRect.height);
      }
    });
    observer.observe(element);

    return () => observer.disconnect();
  }, []);

  return { ref, height };
}
