import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";

const markDone = vi.fn<(id: number) => Promise<View>>();
const openPreferences = vi.fn<() => Promise<void>>();
const quit = vi.fn<() => Promise<void>>();
const hideDropdown = vi.fn<() => Promise<void>>();
const getView = vi.fn<() => Promise<View>>();
const onViewChanged = vi.fn<(f: (v: View) => void) => Promise<() => void>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    markDone: (id: number) => markDone(id),
    openPreferences: () => openPreferences(),
    quit: () => quit(),
    hideDropdown: () => hideDropdown(),
    getView: () => getView(),
    onViewChanged: (f: (v: View) => void) => onViewChanged(f),
  },
}));

const { Dropdown, DropdownContent } = await import("./Dropdown");

function person(p: Partial<Person>): Person {
  return {
    id: 1,
    name: "Ada",
    frequencyDays: 30,
    lastConnected: "2026-10-01",
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
    settings: { dayToggle: "weekendsOnly", notifications: true },
    unreadableFile: null,
  };
}

/** Clicks and lets the command's promise settle. */
async function click(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

const button = (name: string) => screen.getByRole("button", { name });

afterEach(cleanup);

beforeEach(() => {
  for (const f of [markDone, openPreferences, quit, hideDropdown, getView, onViewChanged]) f.mockReset();
  markDone.mockResolvedValue(view([]));
  openPreferences.mockResolvedValue();
  quit.mockResolvedValue();
  hideDropdown.mockResolvedValue();
  onViewChanged.mockResolvedValue(() => {});
});

describe("DropdownContent", () => {
  it("lists who's due with a Done button, and who's coming up", () => {
    render(
      <DropdownContent
        view={view([
          person({ id: 1, name: "Ada" }),
          person({ id: 2, name: "Bo", overdue: true }),
          person({ id: 3, name: "Cy", due: false, notificationDate: "2026-12-14", nextTarget: "2026-12-19" }),
        ])}
      />,
    );
    expect(screen.getAllByTestId("due-person")).toHaveLength(2);
    expect(screen.getByText("Aim for Sat, Nov 21")).toBeTruthy();
    expect(screen.getByText("Missed last time · aim for Sat, Nov 21")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Mark Ada as done" })).toBeTruthy();
    expect(screen.getAllByTestId("upcoming-person")).toHaveLength(1);
    expect(screen.getByText("Reminder in 4 weeks · Mon, Dec 14")).toBeTruthy();
  });

  it("says when everyone is caught up", () => {
    render(<DropdownContent view={view([person({ due: false, notificationDate: "2026-12-14" })])} />);
    expect(screen.getByText("You're all caught up.")).toBeTruthy();
  });

  it("points a new user to Preferences", () => {
    render(<DropdownContent view={view([])} />);
    expect(screen.getByText("Nobody here yet")).toBeTruthy();
  });

  it("says where an unreadable file went", () => {
    render(<DropdownContent view={{ ...view([]), unreadableFile: "/x/friends.json.unreadable-1" }} />);
    expect(screen.getByRole("alert").textContent).toContain("friends.json.unreadable-1");
  });

  it("opens Preferences and quits from the footer", async () => {
    render(<DropdownContent view={view([])} />);
    await click(button("Preferences…"));
    expect(openPreferences).toHaveBeenCalledTimes(1);
    expect(quit).not.toHaveBeenCalled();
    await click(button("Quit Friends"));
    expect(quit).toHaveBeenCalledTimes(1);
  });
});

describe("DropdownContent focus after Done", () => {
  const ADA = person({ id: 1, name: "Ada" });
  const BO = person({ id: 2, name: "Bo" });
  const CY = person({ id: 3, name: "Cy" });
  const notDue = (p: Person) => ({ ...p, due: false, lastConnected: "2026-11-18", notificationDate: "2026-12-14" });

  /** Focuses a person's Done button and presses it, as a keyboard user would. */
  async function pressDone(name: string) {
    const done = button(`Mark ${name} as done`);
    done.focus();
    await click(done);
  }

  it("moves focus to the Done button of the person who took the row's place", async () => {
    const { rerender } = render(<DropdownContent view={view([ADA, BO, CY])} />);
    await pressDone("Ada");
    expect(markDone).toHaveBeenCalledWith(1);
    rerender(<DropdownContent view={view([BO, CY, notDue(ADA)])} />);
    expect(document.activeElement).toBe(button("Mark Bo as done"));
  });

  it("moves focus to the Done button before it when the last row was done", async () => {
    const { rerender } = render(<DropdownContent view={view([ADA, BO, CY])} />);
    await pressDone("Cy");
    rerender(<DropdownContent view={view([ADA, BO, notDue(CY)])} />);
    expect(document.activeElement).toBe(button("Mark Bo as done"));
  });

  it("waits for a view without the person before moving focus", async () => {
    const { rerender } = render(<DropdownContent view={view([ADA, BO])} />);
    await pressDone("Ada");
    // A view from before the Done arrives first: Ada is still due, so nothing moves yet.
    rerender(<DropdownContent view={view([ADA, BO])} />);
    expect(document.activeElement).toBe(button("Mark Ada as done"));
    rerender(<DropdownContent view={view([BO, notDue(ADA)])} />);
    expect(document.activeElement).toBe(button("Mark Bo as done"));
  });

  it("moves focus to the caught-up message when nobody is left", async () => {
    const { rerender } = render(<DropdownContent view={view([ADA])} />);
    await pressDone("Ada");
    rerender(<DropdownContent view={view([notDue(ADA)])} />);
    expect(document.activeElement).toBe(screen.getByText("You're all caught up."));
  });

  it("leaves focus where the user moved it", async () => {
    const { rerender } = render(<DropdownContent view={view([ADA, BO])} />);
    await pressDone("Ada");
    button("Preferences…").focus();
    rerender(<DropdownContent view={view([BO, notDue(ADA)])} />);
    expect(document.activeElement).toBe(button("Preferences…"));
  });

  it("leaves focus alone when Done fails, and shows the error in the row", async () => {
    markDone.mockRejectedValue({ kind: "invalid", message: "That person is gone." });
    const { rerender } = render(<DropdownContent view={view([ADA, BO])} />);
    await pressDone("Ada");
    expect(screen.getByRole("alert").textContent).toBe("That person is gone.");
    // Ada leaves the list some other way later (deleted in Preferences): that isn't the failed Done's move.
    act(() => button("Mark Ada as done").blur());
    rerender(<DropdownContent view={view([BO])} />);
    expect(document.activeElement).toBe(document.body);
  });
});

describe("Dropdown", () => {
  it("shows Loading… until the view arrives, then today's date and the people", async () => {
    let resolve!: (v: View) => void;
    getView.mockReturnValue(new Promise((r) => (resolve = r)));
    render(<Dropdown />);
    expect(screen.getByText("Loading…")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
    await act(async () => resolve(view([person({ id: 1, name: "Ada" })])));
    expect(screen.queryByText("Loading…")).toBeNull();
    expect(screen.getByText("Wed, Nov 18")).toBeTruthy();
    expect(screen.getByTestId("due-person").textContent).toContain("Ada");
  });

  it("shows why the view couldn't be loaded", async () => {
    getView.mockRejectedValue({ kind: "storage", message: "Couldn't read your saved list." });
    render(<Dropdown />);
    await act(async () => {});
    expect(screen.getByRole("alert").textContent).toBe("Couldn't read your saved list.");
    expect(screen.queryByText("Loading…")).toBeNull();
  });

  it("hides the dropdown on Escape, and only on Escape", async () => {
    getView.mockResolvedValue(view([]));
    const { unmount } = render(<Dropdown />);
    await act(async () => {});
    fireEvent.keyDown(window, { key: "Enter" });
    expect(hideDropdown).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(hideDropdown).toHaveBeenCalledTimes(1);
    // The listener goes away with the dropdown.
    unmount();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(hideDropdown).toHaveBeenCalledTimes(1);
  });
});
