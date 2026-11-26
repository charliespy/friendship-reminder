/**
 * The first-run guide, on a new Mac or iPhone: a sign-in page (with "Not now"), then adding people from
 * Contacts (with "Skip"). Each step is shown once; the core remembers which are behind you.
 */
import { type ReactNode } from "react";
import { api, type Account, type GuideStep, type View } from "./api";
import { ContactPicker } from "./ContactPicker";
import { CancelSignInButton, SignInWithAppleButton, waitingText } from "./SignInButtons";
import { useAccount } from "./useAccount";
import { useAction } from "./useView";
import appIcon from "../assets/icon.svg";

type Step = "signIn" | "waitForList" | "import";

/**
 * The step to show now, or null for the app itself. The core keeps which steps are behind you, and
 * the app marks them itself when they don't apply (signed in already, or no account server; people
 * in the list): this only picks the page. `null` account: not known yet.
 */
export function guideStep(view: View, account: Account | null): Step | null {
  const { guide } = view;
  if (!guide.signIn && account?.status === "signedOut") return "signIn";
  if (guide.import || view.people.length > 0) return null;
  // Signed in on a second device: the list may be on its way down from the account.
  if (account?.status === "signedIn" && account.sync?.lastSynced == null && !account.sync?.error) {
    return "waitForList";
  }
  return "import";
}

/**
 * Shows the guide over the app while it has a step left, else the app (`children`). Waits for the
 * account before deciding, so the sign-in page doesn't flash up for someone signed in.
 */
export function WithGuide({ view, phone = false, children }: { view: View; phone?: boolean; children: ReactNode }) {
  const signIn = useAccount();
  const { account } = signIn;
  // "Not now" and "Skip": saved by the core, with an error shown if that fails.
  const finish = useAction();
  const done = (step: GuideStep) => finish.run(() => api.finishGuide(step));
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
          <SignInStep phone={phone} signIn={signIn} onSkip={() => done("signIn")} />
        ) : step === "waitForList" ? (
          <WaitStep onSkip={() => done("import")} />
        ) : (
          <ImportStep onSkip={() => done("import")} />
        )}
        {finish.error && (
          <p className="error guide-error" role="alert">
            {finish.error}
          </p>
        )}
      </div>
    </div>
  );
}

function SignInStep({
  phone,
  signIn,
  onSkip,
}: {
  phone: boolean;
  signIn: ReturnType<typeof useAccount>;
  onSkip: () => void;
}) {
  const { attempt, busy, error, cancelError } = signIn;
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
              {waitingText(phone)}
            </p>
            <CancelSignInButton signIn={signIn} className="guide-button" />
          </>
        ) : (
          <>
            <SignInWithAppleButton signIn={signIn} className="guide-button" />
            <button className="btn btn-quiet guide-button" disabled={busy} onClick={onSkip}>
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

function WaitStep({ onSkip }: { onSkip: () => void }) {
  return (
    <section className="guide-step" aria-labelledby="guide-title">
      <h1 id="guide-title">Getting your list</h1>
      <p className="guide-lead" role="status">
        Friendship Reminder is bringing down the people you added on your other devices.
      </p>
      <div className="guide-actions">
        <button className="btn btn-quiet guide-button" onClick={onSkip}>
          Skip
        </button>
      </div>
    </section>
  );
}

function ImportStep({ onSkip }: { onSkip: () => void }) {
  return (
    <section className="guide-step guide-step-wide" aria-labelledby="guide-title">
      <h1 id="guide-title">Who do you want to keep up with?</h1>
      <p className="guide-lead">
        Add them from your contacts, or skip this and add people by name whenever you like.
      </p>
      {/* Adding people finishes this step by itself (the core marks it once the list has people). */}
      <ContactPicker closeLabel="Skip" onClose={onSkip} onAdded={() => {}} />
    </section>
  );
}
