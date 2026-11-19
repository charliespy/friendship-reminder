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
    const unlisten = api.onViewChanged((v) => live && setView(v));
    api
      .getView()
      .then((v) => live && setView(v))
      .catch((e) => live && setLoadError(errorMessage(e)));
    // Back from sleep or after midnight the dates move on: ask again when the window shows.
    const refresh = () => {
      if (document.visibilityState === "visible") api.getView().then((v) => live && setView(v), () => {});
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
