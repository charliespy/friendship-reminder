import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Account } from "./api";

const getAccount = vi.fn<() => Promise<Account>>();
const signInWithApple = vi.fn<() => Promise<Account>>();
const signOut = vi.fn<() => Promise<Account>>();
const cancelSignIn = vi.fn<() => Promise<Account>>();
let pushAccount: (a: Account) => void = () => {};

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    getAccount: () => getAccount(),
    signInWithApple: () => signInWithApple(),
    signOut: () => signOut(),
    cancelSignIn: () => cancelSignIn(),
    onAccountChanged: (f: (a: Account) => void) => {
      pushAccount = f;
      return Promise.resolve(() => {});
    },
  },
}));

const { AccountSection } = await import("./AccountSection");

async function open(a: Account) {
  getAccount.mockResolvedValue(a);
  render(<AccountSection />);
  await act(async () => {});
}

const signInButton = () => screen.getByRole("button", { name: /Sign in with Apple/ });

beforeEach(() => {
  getAccount.mockReset();
  signInWithApple.mockReset();
  signOut.mockReset();
});
afterEach(cleanup);

describe("AccountSection", () => {
  it("signs in with Apple and shows who is signed in", async () => {
    await open({ status: "signedOut", warning: null });
    signInWithApple.mockResolvedValue({ status: "signedIn", email: "ana@example.com", warning: null });
    await act(async () => fireEvent.click(signInButton()));
    expect(screen.getByText("Signed in with Apple")).toBeTruthy();
    expect(screen.getByTestId("account-email").textContent).toBe("ana@example.com");
  });

  it("waits for the browser, and Cancel stops waiting", async () => {
    await open({ status: "signedOut", warning: null });
    let finish: (a: Account) => void = () => {};
    signInWithApple.mockReturnValue(new Promise((r) => (finish = r)));
    cancelSignIn.mockImplementation(async () => {
      finish({ status: "signedOut", warning: null });
      return { status: "signedOut", warning: null };
    });
    await act(async () => fireEvent.click(signInButton()));
    expect(screen.getByText(/Finish signing in with Apple in your browser/)).toBeTruthy();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Cancel" })));
    expect(cancelSignIn).toHaveBeenCalled();
    expect((signInButton() as HTMLButtonElement).disabled).toBe(false);
  });

  it("signs out", async () => {
    await open({ status: "signedIn", email: null, warning: null });
    expect(screen.queryByTestId("account-email")).toBeNull();
    signOut.mockResolvedValue({ status: "signedOut", warning: null });
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Sign out" })));
    expect(signInButton()).toBeTruthy();
  });

  it("explains why a build can't sign in, with the button off", async () => {
    await open({ status: "unavailable", reason: "This copy of Friends isn't signed for Sign in with Apple." });
    expect((signInButton() as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/isn't signed for Sign in with Apple/)).toBeTruthy();
  });

  it("shows a failed sign-in and keeps the button", async () => {
    await open({ status: "signedOut", warning: null });
    signInWithApple.mockRejectedValue("Couldn't reach the Friends account. Check your internet connection.");
    await act(async () => fireEvent.click(signInButton()));
    expect(screen.getByRole("alert").textContent).toMatch(/Couldn't reach/);
    expect((signInButton() as HTMLButtonElement).disabled).toBe(false);
  });

  it("keeps a newer change when the first read answers late", async () => {
    let answer: (a: Account) => void = () => {};
    getAccount.mockReturnValue(new Promise((r) => (answer = r)));
    render(<AccountSection />);
    act(() => pushAccount({ status: "signedOut", warning: null }));
    await act(async () => answer({ status: "signedIn", email: "ana@example.com", warning: null }));
    expect(signInButton()).toBeTruthy();
  });

  it("shows a warning about the sign-in", async () => {
    await open({ status: "signedIn", email: null, warning: "Couldn't save your sign-in in the Keychain." });
    expect(screen.getByText(/Couldn't save your sign-in/)).toBeTruthy();
  });

  it("follows changes made elsewhere, like the check at launch", async () => {
    await open({ status: "signedIn", email: "ana@example.com", warning: null });
    act(() => pushAccount({ status: "signedOut", warning: null }));
    expect(signInButton()).toBeTruthy();
  });
});
