/** Preferences › Settings › Account: Sign in with Apple, and who is signed in. */
import { useEffect, useState } from "react";
import { api, errorMessage, type Account } from "./api";
import { useAction } from "./useView";

export function AccountSection() {
  const [account, setAccount] = useState<Account | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const action = useAction();
  // Waiting for the browser, where the sign-in happens.
  const [inBrowser, setInBrowser] = useState(false);

  useEffect(() => {
    let live = true;
    // As in useView: every update and request takes a number, and a reply older than what's shown is
    // dropped, so a slow get_account can't undo a newer account-changed (the check at launch).
    let seq = 0;
    let shown = 0;
    const apply = (n: number, a: Account) => {
      if (!live || n < shown) return;
      shown = n;
      setAccount(a);
    };
    const unlisten = api.onAccountChanged((a) => apply(++seq, a));
    const first = ++seq;
    api.getAccount().then(
      (a) => apply(first, a),
      (e) => live && setLoadError(errorMessage(e)),
    );
    return () => {
      live = false;
      unlisten.then(
        (f) => f(),
        () => {},
      );
    };
  }, []);

  const run = (f: () => Promise<Account>) => action.run(async () => setAccount(await f()));

  const signIn = async () => {
    setInBrowser(true);
    try {
      await run(api.signInWithApple);
    } finally {
      setInBrowser(false);
    }
  };

  return (
    <div className="setting-group">
      <h3 className="section-label">Account</h3>
      <div className="choice-list">
        {account?.status === "signedIn" ? (
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
        ) : inBrowser ? (
          <div className="account-row">
            <span className="choice-text">Finish signing in with Apple in your browser.</span>
            {/* Cancelling makes the waiting sign-in stop, which ends the busy state. */}
            <button className="btn btn-outline" onClick={() => void api.cancelSignIn()}>
              Cancel
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
      {action.error && <p className="error" role="alert">{action.error}</p>}
    </div>
  );
}
