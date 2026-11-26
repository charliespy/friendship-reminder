/** The Friendship Reminder account, kept fresh, and signing in and out of it. */
import { useEffect, useRef, useState } from "react";
import { api, errorMessage, type Account } from "./api";
import { useAction } from "./useView";

export function useAccount() {
  const [account, setAccount] = useState<Account | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [cancelError, setCancelError] = useState<string | null>(null);
  // As in useView: every update and request takes a number, and a reply older than what's shown is
  // dropped, so a slow reply can't undo a newer account-changed (the check at launch, say).
  const order = useRef({ seq: 0, shown: 0, live: false });
  const apply = (n: number, a: Account) => {
    const o = order.current;
    if (!o.live || n < o.shown) return;
    o.shown = n;
    setAccount(a);
  };
  const action = useAction();
  // The sign-in waiting for the browser (or the sheet on iPhone); Cancel names it.
  const [attempt, setAttempt] = useState<string | null>(null);

  useEffect(() => {
    const o = order.current;
    o.live = true;
    const unlisten = api.onAccountChanged((a) => apply(++o.seq, a));
    const first = ++o.seq;
    api.getAccount().then(
      (a) => apply(first, a),
      (e) => o.live && setLoadError(errorMessage(e)),
    );
    return () => {
      o.live = false;
      unlisten.then(
        (f) => f(),
        () => {},
      );
    };
  }, []);

  // A command's reply takes its number when the request goes out, so a newer event still wins.
  const run = (f: () => Promise<Account>) =>
    action.run(async () => {
      const n = ++order.current.seq;
      apply(n, await f());
    });

  const signIn = async () => {
    const id = crypto.randomUUID();
    setAttempt(id);
    setCancelError(null);
    try {
      await run(() => api.signInWithApple(id));
    } finally {
      setAttempt(null);
    }
  };

  const cancel = () => {
    if (attempt) api.cancelSignIn(attempt).catch((err) => setCancelError(errorMessage(err)));
  };

  const signOut = () => run(api.signOut);

  return { account, loadError, cancelError, attempt, busy: action.busy, error: action.error, signIn, cancel, signOut };
}
