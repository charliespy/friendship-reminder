import { useCallback, useEffect, useState } from "react";
import { api, errorMessage, type View } from "./api";

/**
 * The app's current view, kept fresh: loaded once, then replaced whenever any window changes something
 * (the Rust side sends the new view to every window).
 */
export function useView() {
  const [view, setView] = useState<View | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    // Every update and request takes a number; a reply older than what's shown is dropped, so a
    // slow get_view can't undo a newer view-changed (e.g. a Done right after the window opens).
    let seq = 0;
    let shown = 0;
    const apply = (n: number, v: View) => {
      if (!live || n < shown) return;
      shown = n;
      setView(v);
    };
    const unlisten = api.onViewChanged((v) => apply(++seq, v));
    const first = ++seq;
    api
      .getView()
      .then((v) => apply(first, v))
      .catch((e) => live && setLoadError(errorMessage(e)));
    // Back from sleep or after midnight the dates move on: ask again when the window shows or comes
    // forward. Both events fire when a window shows, so a request already on its way covers both.
    let asking = false;
    const refresh = () => {
      if (asking || document.visibilityState !== "visible") return;
      asking = true;
      const n = ++seq;
      api.getView().then(
        (v) => apply(n, v),
        () => {},
      ).finally(() => (asking = false));
    };
    document.addEventListener("visibilitychange", refresh);
    window.addEventListener("focus", refresh);
    return () => {
      live = false;
      unlisten.then((f) => f(), () => {});
      document.removeEventListener("visibilitychange", refresh);
      window.removeEventListener("focus", refresh);
    };
  }, []);

  return { view, loadError, setView };
}

/**
 * Runs a command and keeps its error message for the form that started it.
 * `run` resolves to true when the command worked.
 */
export function useAction() {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const run = useCallback(async (f: () => Promise<unknown>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      await f();
      return true;
    } catch (e) {
      setError(errorMessage(e));
      return false;
    } finally {
      setBusy(false);
    }
  }, []);
  return { error, setError, busy, run };
}
