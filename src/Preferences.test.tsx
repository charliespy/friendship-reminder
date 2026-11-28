import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";

const getView = vi.fn<() => Promise<View>>();
const onViewChanged = vi.fn<(f: (v: View) => void) => Promise<() => void>>();
const getLaunchAtLogin = vi.fn<() => Promise<boolean>>();
const setDayToggle = vi.fn<(t: string, d?: string[]) => Promise<View>>();
const takeShownPerson = vi.fn<() => Promise<number | null>>();
const onShowPerson = vi.fn<(f: () => void) => Promise<() => void>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    getView: () => getView(),
    onViewChanged: (f: (v: View) => void) => onViewChanged(f),
    getLaunchAtLogin: () => getLaunchAtLogin(),
    setDayToggle: (t: string, d?: string[]) => setDayToggle(t, d),
    getAccount: () => Promise.resolve({ status: "signedOut", warning: null }),
    onAccountChanged: () => Promise.resolve(() => {}),
    takeShownPerson: () => takeShownPerson(),
    onShowPerson: (f: () => void) => onShowPerson(f),
  },
}));

const { Preferences } = await import("./Preferences");

function person(p: Partial<Person>): Person {
  return {
    suggestedEarlier: null,
    suggestedToday: false,
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
    guide: { signIn: true, import: true },
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
  for (const f of [getView, onViewChanged, getLaunchAtLogin, takeShownPerson, onShowPerson]) f.mockReset();
  onShowPerson.mockResolvedValue(() => {});
  onViewChanged.mockResolvedValue(() => {});
  takeShownPerson.mockResolvedValue(null);
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

  it("opened from a contact card's link, shows that person in People", async () => {
    takeShownPerson.mockResolvedValueOnce(2);
    await open(view([person({ id: 1 }), person({ id: 2, name: "Lena Fischer" })]));
    expect(tab(/^People/).getAttribute("aria-selected")).toBe("true");
    const rows = screen.getAllByTestId("person-row");
    expect(rows.map((r) => r.className.includes("is-highlighted"))).toEqual([false, true]);
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

  it("moves between tabs with the arrow keys, Home and End, and keeps only the selected tab in the Tab order", async () => {
    await open(view([]));
    const selected = () => screen.getAllByRole("tab").map((t) => t.getAttribute("aria-selected") === "true");
    const tabStops = () => screen.getAllByRole("tab").map((t) => t.tabIndex);
    const people = tab(/^People/);
    const blocked = tab(/^Blocked dates/);
    const settings = tab(/^Settings/);
    expect(tabStops()).toEqual([0, -1, -1]);

    people.focus();
    fireEvent.keyDown(people, { key: "ArrowDown" });
    expect(document.activeElement).toBe(blocked);
    expect(selected()).toEqual([false, true, false]);
    expect(panel("blocked").hidden).toBe(false);
    expect(tabStops()).toEqual([-1, 0, -1]);

    fireEvent.keyDown(blocked, { key: "ArrowUp" });
    expect(document.activeElement).toBe(people);
    expect(selected()).toEqual([true, false, false]);

    fireEvent.keyDown(people, { key: "End" });
    expect(document.activeElement).toBe(settings);
    expect(selected()).toEqual([false, false, true]);

    // Past the last tab, Down wraps round to the first; before the first, Up to the last.
    fireEvent.keyDown(settings, { key: "ArrowDown" });
    expect(document.activeElement).toBe(people);
    fireEvent.keyDown(people, { key: "ArrowUp" });
    expect(document.activeElement).toBe(settings);

    fireEvent.keyDown(settings, { key: "Home" });
    expect(document.activeElement).toBe(people);
    expect(selected()).toEqual([true, false, false]);
    expect(tabStops()).toEqual([0, -1, -1]);

    // With Command or Option held the keys keep their usual meaning.
    fireEvent.keyDown(people, { key: "ArrowDown", metaKey: true });
    fireEvent.keyDown(people, { key: "End", altKey: true });
    expect(document.activeElement).toBe(people);
    expect(selected()).toEqual([true, false, false]);
  });

  it("moves from the focused tab when a contact card's link selected another", async () => {
    let linkCame = () => {};
    onShowPerson.mockImplementation((f) => {
      linkCame = f;
      return Promise.resolve(() => {});
    });
    await open(view([MAYA]));
    const settings = tab(/^Settings/);
    fireEvent.click(settings);
    settings.focus();
    takeShownPerson.mockResolvedValueOnce(1);
    await act(async () => linkCame());
    expect(tab(/^People/).getAttribute("aria-selected")).toBe("true");
    // Down from Settings, where focus is, wraps round to People; not on from People to Blocked dates.
    fireEvent.keyDown(settings, { key: "ArrowDown" });
    expect(document.activeElement).toBe(tab(/^People/));
    expect(tab(/^People/).getAttribute("aria-selected")).toBe("true");
  });

  it("shows how many people are due on the People tab, and says what the number counts", async () => {
    await open(view([MAYA, THEO, LENA]));
    expect(tab(/^People/).textContent).toBe("People2");
    expect(screen.getByRole("tab", { name: "People, 2 due" })).toBe(tab(/^People/));
  });

  it("shows no count when nobody is due", async () => {
    await open(view([LENA]));
    expect(tab(/^People/).textContent).toBe("People");
    expect(screen.getByRole("tab", { name: "People" })).toBe(tab(/^People/));
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

  it("waits for Save before changing the day setting, and saves the days with it", async () => {
    setDayToggle.mockReset();
    setDayToggle.mockResolvedValue(view([]));
    await open(view([MAYA]));
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    const save = () => screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(save().disabled).toBe(true);
    expect(screen.queryByRole("group", { name: "Days you're free" })).toBeNull();
    // Pick Custom day: the days show, nothing is sent yet.
    await act(async () => fireEvent.click(screen.getByLabelText(/Custom day/)));
    expect(screen.getByRole("group", { name: "Days you're free" })).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBe("Not saved yet.");
    const day = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;
    await act(async () => fireEvent.click(day("Friday")));
    expect(day("Friday").getAttribute("aria-pressed")).toBe("false");
    expect(setDayToggle).not.toHaveBeenCalled();
    await act(async () => fireEvent.click(save()));
    expect(setDayToggle).toHaveBeenCalledWith("custom", ["mon", "tue", "wed", "thu", "sat", "sun"]);
  });

  it("keeps showing the choice just saved until the new view arrives", async () => {
    setDayToggle.mockReset();
    setDayToggle.mockResolvedValue(view([]));
    await open(view([MAYA]));
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    await act(async () => fireEvent.click(screen.getByLabelText(/Weekdays only/)));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Save" })));
    // The command returned; the view with the new setting hasn't come yet.
    expect((screen.getByLabelText(/Weekdays only/) as HTMLInputElement).checked).toBe(true);
  });

  it("keeps days picked under Custom day while another choice is tried", async () => {
    await open(view([MAYA]));
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    await act(async () => fireEvent.click(screen.getByLabelText(/Custom day/)));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Monday" })));
    await act(async () => fireEvent.click(screen.getByLabelText(/Weekends only/)));
    await act(async () => fireEvent.click(screen.getByLabelText(/Custom day/)));
    expect(screen.getByRole("button", { name: "Monday" }).getAttribute("aria-pressed")).toBe("false");
  });

  it("puts the saved setting back on Discard", async () => {
    setDayToggle.mockReset();
    await open(view([MAYA]));
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Settings" })));
    await act(async () => fireEvent.click(screen.getByLabelText(/Weekdays only/)));
    expect((screen.getByLabelText(/Weekdays only/) as HTMLInputElement).checked).toBe(true);
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Discard" })));
    expect((screen.getByLabelText(/Weekends only/) as HTMLInputElement).checked).toBe(true);
    expect(screen.queryByRole("button", { name: "Discard" })).toBeNull();
    expect(setDayToggle).not.toHaveBeenCalled();
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
