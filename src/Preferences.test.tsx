import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";

const getView = vi.fn<() => Promise<View>>();
const onViewChanged = vi.fn<(f: (v: View) => void) => Promise<() => void>>();
const getLaunchAtLogin = vi.fn<() => Promise<boolean>>();
const setDayToggle = vi.fn<(t: string) => Promise<View>>();
const setCustomDays = vi.fn<(d: string[]) => Promise<View>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    getView: () => getView(),
    onViewChanged: (f: (v: View) => void) => onViewChanged(f),
    getLaunchAtLogin: () => getLaunchAtLogin(),
    setDayToggle: (t: string) => setDayToggle(t),
    setCustomDays: (d: string[]) => setCustomDays(d),
    getAccount: () => Promise.resolve({ status: "signedOut", warning: null }),
    onAccountChanged: () => Promise.resolve(() => {}),
  },
}));

const { Preferences } = await import("./Preferences");

function person(p: Partial<Person>): Person {
  return {
    suggestedDay: null,
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

const MAYA = person({ id: 1, name: "Maya Chen" });
const THEO = person({ id: 2, name: "Theo Marsh" });
const LENA = person({
  id: 3,
  name: "Lena Fischer",
  frequencyDays: 365,
  lastConnected: "2026-03-02",
  nextTarget: "2027-03-06",
  notificationDate: "2027-03-01",
  due: false,
});

function view(people: Person[], unreadableFile: string | null = null): View {
  return {
    today: "2026-11-18",
    people,
    blocked: [],
    settings: { dayToggle: "weekendsOnly", customDays: ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], notifications: true },
    unreadableFile,
  };
}

/** Renders Preferences and lets the first view arrive. */
async function open(v: View) {
  getView.mockResolvedValue(v);
  render(<Preferences />);
  await act(async () => {});
}

const tab = (name: RegExp) => screen.getByRole("tab", { name });
const panel = (id: string) => document.getElementById(`pane-${id}`)!;

afterEach(cleanup);

beforeEach(() => {
  for (const f of [getView, onViewChanged, getLaunchAtLogin]) f.mockReset();
  onViewChanged.mockResolvedValue(() => {});
  getLaunchAtLogin.mockResolvedValue(false);
});

describe("Preferences", () => {
  it("shows Loading… until the view arrives", () => {
    getView.mockReturnValue(new Promise(() => {}));
    render(<Preferences />);
    expect(screen.getByText("Loading…")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByRole("tabpanel")).toBeNull();
  });

  it("shows why the view couldn't be loaded", async () => {
    getView.mockRejectedValue({ kind: "storage", message: "Couldn't read your saved list." });
    render(<Preferences />);
    await act(async () => {});
    expect(screen.getByRole("alert").textContent).toBe("Couldn't read your saved list.");
  });

  it("starts on People, and switches tabs and panels together", async () => {
    await open(view([]));
    expect(screen.getAllByRole("tab").map((t) => t.textContent)).toEqual(["People", "Blocked dates", "Settings"]);
    const shown = () => ({
      people: [tab(/^People/).getAttribute("aria-selected"), panel("people").hidden],
      blocked: [tab(/^Blocked dates/).getAttribute("aria-selected"), panel("blocked").hidden],
      settings: [tab(/^Settings/).getAttribute("aria-selected"), panel("settings").hidden],
    });
    expect(shown()).toEqual({ people: ["true", false], blocked: ["false", true], settings: ["false", true] });

    fireEvent.click(tab(/^Blocked dates/));
    expect(shown()).toEqual({ people: ["false", true], blocked: ["true", false], settings: ["false", true] });

    fireEvent.click(tab(/^Settings/));
    expect(shown()).toEqual({ people: ["false", true], blocked: ["false", true], settings: ["true", false] });

    // Each tab names the panel it controls, and each panel the tab that labels it.
    for (const id of ["people", "blocked", "settings"]) {
      expect(document.getElementById(`tab-${id}`)!.getAttribute("aria-controls")).toBe(`pane-${id}`);
      expect(panel(id).getAttribute("aria-labelledby")).toBe(`tab-${id}`);
    }
  });

  it("shows how many people are due on the People tab", async () => {
    await open(view([MAYA, THEO, LENA]));
    expect(tab(/^People/).textContent).toBe("People2");
  });

  it("shows no count when nobody is due", async () => {
    await open(view([LENA]));
    expect(tab(/^People/).textContent).toBe("People");
  });

  it("keeps a half-filled form when switching tabs and back", async () => {
    await open(view([]));
    fireEvent.click(screen.getByRole("button", { name: "Add person" }));
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Nora Quill" } });
    fireEvent.click(tab(/^Blocked dates/));
    fireEvent.click(tab(/^People/));
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("Nora Quill");
  });

  it("says where an unreadable file went", async () => {
    await open(view([], "/x/friends.json.unreadable-1"));
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("couldn't be read");
    expect(alert.textContent).toContain("/x/friends.json.unreadable-1");
  });

  it("shows no banner when the file was read", async () => {
    await open(view([MAYA]));
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("shows the day picker only under Custom day, and flips one day at a time", async () => {
    setCustomDays.mockReset();
    setCustomDays.mockResolvedValue(view([]));
    await open(view([MAYA]));
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    expect(screen.queryByRole("group", { name: "Days you're free" })).toBeNull();
    cleanup();
    const custom = view([MAYA]);
    custom.settings = { ...custom.settings, dayToggle: "custom", customDays: ["tue", "sat"] };
    await open(custom);
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    const days = screen.getByRole("group", { name: "Days you're free" });
    const day = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;
    expect(days.querySelectorAll('[aria-pressed="true"]').length).toBe(2);
    expect(day("Tuesday").getAttribute("aria-pressed")).toBe("true");
    await act(async () => fireEvent.click(day("Thursday")));
    expect(setCustomDays).toHaveBeenCalledWith(["tue", "thu", "sat"]);
    await act(async () => fireEvent.click(day("Saturday")));
    expect(setCustomDays).toHaveBeenLastCalledWith(["tue"]);
  });

  it("keeps the last picked day: at least one is needed", async () => {
    const custom = view([MAYA]);
    custom.settings = { ...custom.settings, dayToggle: "custom", customDays: ["sun"] };
    await open(custom);
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    expect((screen.getByRole("button", { name: "Sunday" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Monday" }) as HTMLButtonElement).disabled).toBe(false);
  });
});
