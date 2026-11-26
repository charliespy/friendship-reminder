import { useCallback, useEffect, useState } from "react";
import { api, errorMessage, type NotificationAccess } from "./api";

/**
 * Whether the app may send notifications, read again whenever the app comes back on screen (the
 * user may have changed it in iOS Settings), and a way to ask. `access` is null until known.
 */
export function useNotificationAccess() {
  const [access, setAccess] = useState<NotificationAccess | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const read = () => {
      if (document.visibilityState !== "visible") return;
      api.notificationAccess().then(
        (a) => live && setAccess(a),
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

  /** Shows iOS's question (the first time), then keeps the answer. */
  const allow = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      setAccess(await api.allowNotifications());
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }, []);

  return { access, allow, busy, error };
}

export type NotificationAccessState = ReturnType<typeof useNotificationAccess>;
