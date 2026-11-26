import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorMessage, type NotificationAccess } from "./api";

/**
 * Whether the app may send notifications, read again whenever the app comes back on screen (the
 * user may have changed it in iOS Settings), and a way to ask. `access` is null until known.
 */
export function useNotificationAccess() {
  const [access, setAccess] = useState<NotificationAccess | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Each read takes a number, and an answer to ask moves it on: an answer older than the latest
  // is dropped, so a slow read can't undo what the user just allowed.
  const latest = useRef(0);

  useEffect(() => {
    let live = true;
    const read = () => {
      if (document.visibilityState !== "visible") return;
      const n = ++latest.current;
      api.notificationAccess().then(
        (a) => live && n === latest.current && setAccess(a),
        // Unknown: nothing offered (turning reminders on in Settings still asks).
        () => {},
      );
    };
    read();
    document.addEventListener("visibilitychange", read);
    return () => {
      live = false;
      document.removeEventListener("visibilitychange", read);
    };
  }, []);

  /** Shows iOS's question (the first time) and keeps the answer; throws when it can't ask. */
  const ask = useCallback(async () => {
    const answer = await api.allowNotifications();
    // The user's answer wins over any read started while iOS was asking.
    ++latest.current;
    setAccess(answer);
  }, []);

  /** `ask`, for a button: busy meanwhile, and the error kept. */
  const allow = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await ask();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }, [ask]);

  return { access, ask, allow, busy, error };
}

export type NotificationAccessState = ReturnType<typeof useNotificationAccess>;
