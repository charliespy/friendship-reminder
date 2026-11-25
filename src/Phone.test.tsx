import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";

const getView = vi.fn<() => Promise<View>>();
const getLaunchAtLogin = vi.fn<() => Promise<boolean>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    getView: () => getView(),
    onViewChanged: () => Promise.resolve(() => {}),
    getLaunchAtLogin: () => getLaunchAtLogin(),
    getAccount: () => Promise.resolve({ status: "signedOut", warning: null }),
    onAccountChanged: () => Promise.resolve(() => {}),
  },
}));

const { Phone } = await import("./Phone");

function person(p: Partial<Person>): Person {
  return {
    suggestedDay: null,
    earliestDone: "2026-01-01",
    id: 1,
    name: "Maya Chen",
    frequencyDays: 30,
    lastConnected: "2026-10-10",
    nextTarget: "2026-11-21",
    notificationDate: "2026-11-16",
    overdue: false,
    due: true,
    ...p,
  };
}

function view(people: Person[]): View {
  return {
    today: "2026-11-18",
    people,
    blocked: [],
    settings: { dayToggle: "weekendsOnly", customDays: ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], notifications: true },
    unreadableFile: null,
  };
}

beforeEach(() => {
  getLaunchAtLogin.mockResolvedValue(false);
});

afterEach(cleanup);

describe("Phone", () => {
  it("opens on Today, with how many are due on its tab", async () => {
    getView.mockResolvedValue(
      view([person({ id: 1 }), person({ id: 2, name: "Theo Marsh" }), person({ id: 3, name: "Lena Fischer", due: false })]),
    );
    render(<Phone />);
    expect(await screen.findByRole("heading", { name: "Today" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Today, 2 due" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getAllByTestId("due-person")).toHaveLength(2);
    expect(screen.getAllByTestId("upcoming-person")).toHaveLength(1);
    // The Mac dropdown's footer (Preferences, Quit) isn't on the phone.
    expect(screen.queryByRole("button", { name: "Quit Friends" })).toBeNull();
  });

  it("points to the People tab when nobody is added yet", async () => {
    getView.mockResolvedValue(view([]));
    render(<Phone />);
    expect(await screen.findByText("Add the people you want to keep up with in People.")).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Today" })).toBeTruthy();
  });

  it("switches tabs, and Settings has no Open at login", async () => {
    getView.mockResolvedValue(view([person({})]));
    render(<Phone />);
    await screen.findByRole("heading", { name: "Today" });
    fireEvent.click(screen.getByRole("tab", { name: "People" }));
    expect(screen.getByRole("tab", { name: "People" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("heading", { name: "People" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "People" }).closest("[role=tabpanel]")?.hasAttribute("hidden")).toBe(false);
    expect(screen.getByRole("heading", { name: "Today", hidden: true }).closest("[role=tabpanel]")?.hasAttribute("hidden")).toBe(true);
    fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
    expect(screen.getByRole("heading", { name: "Settings" })).toBeTruthy();
    expect(screen.queryByText("Open at login")).toBeNull();
    expect(screen.getByText(/They also show in Today either way/)).toBeTruthy();
    expect(getLaunchAtLogin).not.toHaveBeenCalled();
  });
});
