import { useEffect, useRef, type FocusEvent } from "react";

/**
 * When something on screen goes away (`shown` turns false) and takes the focused control with it,
 * focus would be left on the page itself, so the next Tab starts at the top and a screen reader
 * says nothing. This sends it to `target()` instead: what the screen shows now.
 *
 * `shown` may also name what is shown (a page of the guide, say; `false` or `null` for nothing):
 * a change from one to another counts as the old one going away too.
 *
 * Only when focus was inside it (spread the returned props on its outer element): a tap on iPhone
 * doesn't focus a button, and then nothing moves. Never takes focus from somewhere the user moved it.
 *
 * `inPlace`: what replaces it sits where it was (a button swapped for another in the same row), so a
 * change doesn't scroll to it either: the user may have scrolled elsewhere while it waited.
 */
export function useFocusWhenGone(
  shown: boolean | string | null,
  target: () => HTMLElement | null | undefined,
  { inPlace = false }: { inPlace?: boolean } = {},
) {
  const wasShown = useRef(shown);
  // Whether focus is inside. Removing the focused control fires no blur in WebKit (and where it
  // does, the control is detached by then), so this stays true when focus goes away with it;
  // leaving for somewhere else on the page clears it.
  const inside = useRef(false);
  // Runs after every render, so it reads the latest `target`.
  useEffect(() => {
    const nothing = (value: typeof shown) => value === false || value === null;
    const gone = !nothing(wasShown.current) && wasShown.current !== shown;
    wasShown.current = shown;
    if (!gone) return;
    const hadFocus = inside.current;
    const closed = nothing(shown);
    // A new page is still inside: focus a control on it took by itself stays counted.
    if (closed) inside.current = false;
    const lost = !document.activeElement || document.activeElement === document.body;
    // Closing doesn't scroll: the screen may be showing a person a contact card's link opened. A
    // new page brings its target into view, unless it is in the same place.
    if (hadFocus && lost) target()?.focus({ preventScroll: closed || inPlace });
  });
  return {
    onFocus: () => {
      inside.current = true;
    },
    onBlur: (e: FocusEvent) => {
      const to = e.relatedTarget;
      if (to instanceof Node) {
        if (!e.currentTarget.contains(to)) inside.current = false;
        return;
      }
      // To nowhere: a click on the page's background, the control being removed or turned off
      // while it works (Chrome drops focus from a disabled button), or the window losing focus (iOS
      // asking about notifications, the browser opening to sign in), after which focus comes back
      // to the same control. Only the first means focus left.
      const from = e.target;
      queueMicrotask(() => {
        if (from.isConnected && !from.matches(":disabled") && document.hasFocus()) inside.current = false;
      });
    },
  };
}
