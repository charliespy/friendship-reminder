/** The person a link from a contact card opened the app on: the app shows them in People. */
import { useEffect, useRef, useState } from "react";
import { api } from "./api";

/** A person to show, and when the link came (so the same person twice shows twice). */
export interface Shown {
  id: number;
  at: number;
}

/**
 * `onShow` runs as a person comes in (to open the People tab, say).
 *
 * Asks once when the window opens (the link may have started the app) and again whenever the app
 * says a link came in. The app hands each person to one window only.
 */
export function useShownPerson(onShow?: () => void): Shown | null {
  const [shown, setShown] = useState<Shown | null>(null);
  // The latest callback, without asking again each time it changes.
  const callback = useRef(onShow);
  useEffect(() => {
    callback.current = onShow;
  });
  useEffect(() => {
    let live = true;
    const take = () =>
      api.takeShownPerson().then(
        (id) => {
          if (!live || id == null) return;
          setShown({ id, at: Date.now() });
          callback.current?.();
        },
        () => {},
      );
    take();
    const unlisten = api.onShowPerson(take);
    return () => {
      live = false;
      unlisten.then(
        (f) => f(),
        () => {},
      );
    };
  }, []);
  return shown;
}
