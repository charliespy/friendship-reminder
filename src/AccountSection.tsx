/** Preferences › Settings › Account: Sign in with Apple, and who is signed in. */
import { useEffect, useRef, useState } from "react";
import { api, errorMessage, type Account } from "./api";
import { useAction } from "./useView";

export function AccountSection() {
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
  // The sign-in waiting for the browser, where it happens; Cancel names it.
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

  return (
    <div className="setting-group">
      <h3 className="section-label">Account</h3>
      <div className="choice-list">
        {attempt ? (
          <div className="account-row">
            <span className="choice-text">Finish signing in with Apple in your browser.</span>
            {/* Cancelling makes the waiting sign-in stop, which ends the busy state. */}
            <button
              className="btn btn-outline"
              onClick={(e) => {
                // The second click of a double-click on Sign in lands here: not a Cancel.
                if (e.detail > 1) return;
                api.cancelSignIn(attempt).catch((err) => setCancelError(errorMessage(err)));
              }}
            >
              Cancel
            </button>
          </div>
        ) : account?.status === "signedIn" ? (
          <div className="account-row">
            <span>
              <span className="choice-title">Signed in with Apple</span>
              {account.email && (
                <>
                  <br />
                  <span className="choice-text" data-testid="account-email">{account.email}</span>
                </>
              )}
            </span>
            <button className="btn btn-outline" disabled={action.busy} onClick={() => run(api.signOut)}>
              Sign out
            </button>
          </div>
        ) : (
          <div className="account-row">
            <span className="choice-text">
              Sign in to set up your Friends account. Your list stays on this Mac for now.
            </span>
            <button
              className="btn btn-apple"
              disabled={action.busy || account?.status !== "signedOut"}
              onClick={signIn}
            >
              {/* U+F8FF is the Apple logo in Apple's system font. */}
              <span aria-hidden="true" className="apple-logo">{"\uF8FF"}</span>
              Sign in with Apple
            </button>
          </div>
        )}
      </div>
      {account?.status === "unavailable" && <p className="hint">{account.reason}</p>}
      {account?.status !== "unavailable" && account?.warning && <p className="hint">{account.warning}</p>}
      {loadError && <p className="error" role="alert">{loadError}</p>}
      {cancelError && <p className="error" role="alert">{cancelError}</p>}
      {action.error && <p className="error" role="alert">{action.error}</p>}
    </div>
  );
}
