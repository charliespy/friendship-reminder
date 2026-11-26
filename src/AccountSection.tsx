/** Preferences › Settings › Account: Sign in with Apple, and who is signed in. */
import { useEffect, useState } from "react";
import { type SyncStatus } from "./api";
import { timeAgo } from "./format";
import { CancelSignInButton, SignInWithAppleButton, waitingText } from "./SignInButtons";
import { useAccount } from "./useAccount";

/** The line under the account about syncing. */
function syncLine(sync: SyncStatus | null | undefined, now: number): string {
  if (sync?.error) return sync.error;
  if (sync?.lastSynced != null) return `Your list is synced with your account. Last synced ${timeAgo(sync.lastSynced, now)}.`;
  if (sync?.syncing) return "Syncing your list with your account…";
  return "Your list will sync with your account in a moment.";
}

/** The time now, updated every half minute, so "2 minutes ago" keeps up. */
function useNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);
  return now;
}

/** `phone`: signing in happens in a sheet over the app, not the browser. */
export function AccountSection({ phone = false }: { phone?: boolean }) {
  const signIn = useAccount();
  const { account, loadError, cancelError, attempt, busy, error, signOut } = signIn;
  const now = useNow();

  return (
    <div className="setting-group">
      <h3 className="section-label">Account</h3>
      <div className="choice-list">
        {attempt ? (
          <div className="account-row">
            <span className="choice-text">{waitingText(phone)}</span>
            <CancelSignInButton signIn={signIn} />
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
            <button className="btn btn-outline" disabled={busy} onClick={signOut}>
              Sign out
            </button>
          </div>
        ) : (
          <div className="account-row">
            <span className="choice-text">
              Sign in to keep your list in your Friendship Reminder account, ready for your other devices.
            </span>
            <SignInWithAppleButton signIn={signIn} />
          </div>
        )}
      </div>
      {account?.status === "signedIn" && !attempt && (
        <p className={account.sync?.error ? "error" : "hint"} data-testid="sync-status">
          {syncLine(account.sync, now)}
        </p>
      )}
      {account?.status === "unavailable" && <p className="hint">{account.reason}</p>}
      {account?.status !== "unavailable" && account?.warning && <p className="hint">{account.warning}</p>}
      {loadError && <p className="error" role="alert">{loadError}</p>}
      {cancelError && <p className="error" role="alert">{cancelError}</p>}
      {error && <p className="error" role="alert">{error}</p>}
    </div>
  );
}
