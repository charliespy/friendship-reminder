import { useEffect, useRef } from "react";

/**
 * When something on screen goes away (`shown` turns false) and takes the focused control with it,
 * focus would be left on the page itself, so the next Tab starts at the top and a screen reader
 * says nothing. This sends it to `target()` instead: what the screen shows now. Only when focus
 * was lost; never takes it from somewhere the user moved it.
 */
export function useFocusWhenGone(shown: boolean, target: () => HTMLElement | null | undefined) {
  const wasShown = useRef(shown);
  // Runs after every render, so it reads the latest `target`.
  useEffect(() => {
    const gone = wasShown.current && !shown;
    wasShown.current = shown;
    if (!gone) return;
    const active = document.activeElement;
    if (active && active !== document.body) return;
    target()?.focus();
  });
}
