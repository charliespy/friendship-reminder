/** The Sign in with Apple and Cancel buttons, as Settings › Account and the first-run guide show them. */
import type { useAccount } from "./useAccount";

type SignIn = ReturnType<typeof useAccount>;

/** The line shown while a sign-in waits for Apple. */
export function waitingText(phone: boolean): string {
  return phone ? "Finish signing in with Apple." : "Finish signing in with Apple in your browser.";
}

export function SignInWithAppleButton({ signIn, className = "" }: { signIn: SignIn; className?: string }) {
  return (
    <button
      className={`btn btn-apple ${className}`.trim()}
      disabled={signIn.busy || signIn.account?.status !== "signedOut"}
      onClick={signIn.signIn}
    >
      {/* U+F8FF is the Apple logo in Apple's system font. */}
      <span aria-hidden="true" className="apple-logo">{""}</span>
      Sign in with Apple
    </button>
  );
}

/** Stops the sign-in waiting for Apple, which ends the busy state. */
export function CancelSignInButton({ signIn, className = "" }: { signIn: SignIn; className?: string }) {
  return (
    <button
      className={`btn btn-outline ${className}`.trim()}
      onClick={(e) => {
        // The second click of a double-click on Sign in lands here: not a Cancel.
        if (e.detail > 1) return;
        signIn.cancel();
      }}
    >
      Cancel
    </button>
  );
}
