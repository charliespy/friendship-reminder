/**
 * The first-run guide, on a new Mac or iPhone: a sign-in page (with "Not now"), then adding people from
 * Contacts (with "Skip"). Each step is shown once; the core remembers which are behind you.
 */
import { useEffect, useRef, type ReactNode } from "react";
import { api, type Account, type GuideStep, type View } from "./api";
import { ContactPicker } from "./ContactPicker";
import { useAccount } from "./useAccount";
import appIcon from "../assets/icon.svg";

type Step = "signIn" | "waitForList" | "import";

/** The step to show now, or null for the app itself. `null` account: not known yet. */
export function guideStep(view: View, account: Account | null): Step | null {
  const { guide } = view;
  if (guide.signIn && guide.import) return null;
  if (!guide.signIn) {
    // Without an account server there's nothing to sign in to: that step is skipped.
    return account?.status === "signedOut" ? "signIn" : null;
  }
  if (view.people.length > 0) return null;
  // Signed in on a second device: the list may be on its way down from the account.
  if (account?.status === "signedIn" && account.sync?.lastSynced == null && !account.sync?.error) {
    return "waitForList";
  }
  return "import";
}

/** Marks the steps that are behind you without a choice: signed in already, people already listed. */
function useFinishQuietly(view: View | null, account: Account | null) {
  const sent = useRef(new Set<GuideStep>());
  useEffect(() => {
    if (!view || !account) return;
    const finish = (step: GuideStep) => {
      if (sent.current.has(step)) return;
      sent.current.add(step);
      api.finishGuide(step).catch(() => sent.current.delete(step));
    };
    if (!view.guide.signIn && account.status !== "signedOut") finish("signIn");
    if (view.guide.signIn && !view.guide.import && view.people.length > 0) finish("import");
  });
}

/**
 * Shows the guide over the app while it has a step left, else the app (`children`). Waits for the
 * account before deciding, so the sign-in page doesn't flash up for someone signed in.
 */
export function WithGuide({ view, phone = false, children }: { view: View; phone?: boolean; children: ReactNode }) {
  const signIn = useAccount();
  const { account } = signIn;
  useFinishQuietly(view, account);
  const finished = view.guide.signIn && view.guide.import;
  if (finished) return <>{children}</>;
  // The account answers at once; if it can't be read, the guide stays out of the way.
  if (!account) return signIn.loadError ? <>{children}</> : null;
  const step = guideStep(view, account);
  if (!step) return <>{children}</>;
  return (
    <div className={phone ? "phone guide guide-phone" : "guide"}>
      <div className="guide-body">
        {step === "signIn" ? (
          <SignInStep phone={phone} {...signIn} />
        ) : step === "waitForList" ? (
          <WaitStep />
        ) : (
          <ImportStep />
        )}
      </div>
    </div>
  );
}

function skip(step: GuideStep) {
  api.finishGuide(step).catch(() => {});
}

function SignInStep({
  phone,
  account,
  attempt,
  busy,
  error,
  cancelError,
  signIn,
  cancel,
}: { phone: boolean } & ReturnType<typeof useAccount>) {
  return (
    <section className="guide-step" aria-labelledby="guide-title">
      <img className="guide-icon" src={appIcon} alt="" width={64} height={64} />
      <h1 id="guide-title">Welcome to Friendship Reminder</h1>
      <p className="guide-lead">
        It reminds you when it's time to reach out to the people you care about. Sign in to keep your list on your
        Mac and your iPhone.
      </p>
      <div className="guide-actions">
        {attempt ? (
          <>
            <p className="muted" role="status">
              {phone ? "Finish signing in with Apple." : "Finish signing in with Apple in your browser."}
            </p>
            <button className="btn btn-outline guide-button" onClick={(e) => e.detail <= 1 && cancel()}>
              Cancel
            </button>
          </>
        ) : (
          <>
            <button
              className="btn btn-apple guide-button"
              disabled={busy || account?.status !== "signedOut"}
              onClick={signIn}
            >
              {/* U+F8FF is the Apple logo in Apple's system font. */}
              <span aria-hidden="true" className="apple-logo">{""}</span>
              Sign in with Apple
            </button>
            <button className="btn btn-quiet guide-button" disabled={busy} onClick={() => skip("signIn")}>
              Not now
            </button>
          </>
        )}
      </div>
      {error && <p className="error" role="alert">{error}</p>}
      {cancelError && <p className="error" role="alert">{cancelError}</p>}
      <p className="hint guide-note">
        Without an account, your list stays on this {phone ? "iPhone" : "Mac"}. You can sign in later in Settings.
      </p>
    </section>
  );
}

function WaitStep() {
  return (
    <section className="guide-step" aria-labelledby="guide-title">
      <h1 id="guide-title">Getting your list</h1>
      <p className="guide-lead" role="status">
        Friendship Reminder is bringing down the people you added on your other devices.
      </p>
      <div className="guide-actions">
        <button className="btn btn-quiet guide-button" onClick={() => skip("import")}>
          Skip
        </button>
      </div>
    </section>
  );
}

function ImportStep() {
  return (
    <section className="guide-step guide-step-wide" aria-labelledby="guide-title">
      <h1 id="guide-title">Who do you want to keep up with?</h1>
      <p className="guide-lead">
        Pick people from your contacts. Friendship Reminder reads only their names, and adds a link back to the app on
        their cards.
      </p>
      <ContactPicker closeLabel="Skip" onClose={() => skip("import")} onAdded={() => skip("import")} />
    </section>
  );
}
