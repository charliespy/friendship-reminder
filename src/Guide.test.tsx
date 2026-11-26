import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Account, Person, View } from "./api";

const finishGuide = vi.fn<(step: string) => Promise<View>>();
let account: Account = { status: "signedOut", warning: null };

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    finishGuide: (step: string) => finishGuide(step),
    getAccount: () => Promise.resolve(account),
    onAccountChanged: () => Promise.resolve(() => {}),
    listContacts: () => new Promise(() => {}),
  },
}));

const { WithGuide, guideStep } = await import("./Guide");

function view(guide: View["guide"], people: Person[] = []): View {
  return {
    today: "2026-11-18",
    people,
    blocked: [],
    settings: { dayToggle: "weekendsOnly", customDays: ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], notifications: true },
    unreadableFile: null,
    guide,
  };
}

const someone = { id: 1, name: "Maya Chen" } as Person;
const fresh = { signIn: false, import: false };
const signedOut: Account = { status: "signedOut", warning: null };
const signedIn = (lastSynced: number | null): Account => ({
  status: "signedIn",
  email: null,
  warning: null,
  sync: { syncing: lastSynced == null, lastSynced, error: null },
});

afterEach(() => {
  cleanup();
  finishGuide.mockReset();
  account = signedOut;
});

describe("guideStep", () => {
  it("shows the sign-in page first, to someone signed out", () => {
    expect(guideStep(view(fresh), signedOut)).toBe("signIn");
  });

  it("skips signing in when there's no account server", () => {
    expect(guideStep(view(fresh), { status: "unavailable", reason: "" })).toBeNull();
  });

  it("offers Contacts once signing in is behind you, while the list is empty", () => {
    expect(guideStep(view({ signIn: true, import: false }), signedOut)).toBe("import");
    expect(guideStep(view({ signIn: true, import: false }, [someone]), signedOut)).toBeNull();
  });

  it("waits for a signed-in device's first sync before offering Contacts", () => {
    const after = { signIn: true, import: false };
    expect(guideStep(view(after), signedIn(null))).toBe("waitForList");
    expect(guideStep(view(after), signedIn(1000))).toBe("import");
  });

  it("is over once both steps are", () => {
    expect(guideStep(view({ signIn: true, import: true }), signedOut)).toBeNull();
  });
});

describe("WithGuide", () => {
  it("shows the app at once when the guide is done", () => {
    render(<WithGuide view={view({ signIn: true, import: true })}>the app</WithGuide>);
    expect(screen.getByText("the app")).toBeTruthy();
  });

  it("shows the sign-in page in place of the app for a new user", async () => {
    render(<WithGuide view={view(fresh)}>the app</WithGuide>);
    expect(await screen.findByRole("heading", { name: "Welcome to Friendship Reminder" })).toBeTruthy();
    expect(screen.queryByText("the app")).toBeNull();
  });

  it("marks the steps done without asking when they don't apply", async () => {
    account = signedIn(1000);
    finishGuide.mockResolvedValue(view(fresh));
    render(<WithGuide view={view(fresh, [someone])}>the app</WithGuide>);
    await waitFor(() => expect(finishGuide).toHaveBeenCalledWith("signIn"));
    expect(finishGuide).not.toHaveBeenCalledWith("import");
  });
});
