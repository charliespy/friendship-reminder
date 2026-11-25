import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";

const addPerson = vi.fn<(p: unknown) => Promise<View>>();
const updatePerson = vi.fn<(p: unknown) => Promise<View>>();
const deletePeople = vi.fn<(ids: number[]) => Promise<View>>();
const markDone = vi.fn<(id: number) => Promise<View>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    addPerson: (p: unknown) => addPerson(p),
    updatePerson: (p: unknown) => updatePerson(p),
    deletePeople: (ids: number[]) => deletePeople(ids),
    markDone: (id: number) => markDone(id),
  },
}));

const { PeoplePane } = await import("./PeoplePane");

const TODAY = "2026-11-18";

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
const THEO = person({
  id: 2,
  name: "Theo Marsh",
  frequencyDays: 90,
  lastConnected: "2026-09-29",
  nextTarget: "2027-01-02",
  notificationDate: "2026-12-28",
  due: false,
});
const LENA = person({
  id: 3,
  name: "Lena Fischer",
  frequencyDays: 365,
  lastConnected: "2026-03-02",
  nextTarget: "2027-03-06",
  notificationDate: "2027-03-01",
  due: false,
});

function view(people: Person[]): View {
  return {
    today: TODAY,
    people,
    blocked: [],
    settings: { dayToggle: "weekendsOnly", customDays: ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], notifications: true },
    unreadableFile: null,
  };
}

/** Clicks and lets the command's promise settle. */
async function click(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

/** Presses a person's Done, and answers the question it asks: in touch today. */
async function doneToday(name: string) {
  await click(screen.getByRole("button", { name: `Mark ${name} as done` }));
  await click(within(screen.getByRole("dialog")).getByRole("button", { name: /^Today/ }));
}

const button = (name: string) => screen.getByRole("button", { name });
const form = (name: string) => screen.queryByRole("form", { name });

afterEach(cleanup);

beforeEach(() => {
  for (const f of [addPerson, updatePerson, deletePeople, markDone]) {
    f.mockReset();
    f.mockResolvedValue(view([]));
  }
});

describe("PeoplePane", () => {
  it("says when nobody has been added yet", () => {
    render(<PeoplePane view={view([])} />);
    expect(screen.getByText("Nobody yet")).toBeTruthy();
  });

  it("disables Add person while the name is empty or only spaces", async () => {
    render(<PeoplePane view={view([])} />);
    await click(button("Add person"));
    const submit = within(form("Add person")!).getByRole("button", { name: "Add person" }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "   " } });
    expect(submit.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Maya Chen" } });
    expect(submit.disabled).toBe(false);
  });

  it("adds a person with a trimmed name and closes the form", async () => {
    render(<PeoplePane view={view([])} />);
    await click(button("Add person"));
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "  Maya Chen  " } });
    fireEvent.change(screen.getByLabelText("How often"), { target: { value: "90" } });
    fireEvent.change(screen.getByLabelText("Last in touch"), { target: { value: "2026-10-10" } });
    await click(within(form("Add person")!).getByRole("button", { name: "Add person" }));
    expect(addPerson).toHaveBeenCalledWith({ name: "Maya Chen", frequencyDays: 90, lastConnected: "2026-10-10" });
    expect(form("Add person")).toBeNull();
  });

  it("shows an error from the API in the form and keeps it open", async () => {
    addPerson.mockRejectedValue({ kind: "invalid", message: "That name is too long." });
    render(<PeoplePane view={view([])} />);
    await click(button("Add person"));
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Maya Chen" } });
    await click(within(form("Add person")!).getByRole("button", { name: "Add person" }));
    expect(within(form("Add person")!).getByRole("alert").textContent).toBe("That name is too long.");
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("Maya Chen");
  });

  it("leaves out the last-in-touch date when saving an edit that didn't change it", async () => {
    render(<PeoplePane view={view([MAYA, THEO])} />);
    await click(button("Edit Theo Marsh"));
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Theo Marsh-Lee" } });
    await click(button("Save changes"));
    expect(updatePerson).toHaveBeenCalledWith({ id: 2, name: "Theo Marsh-Lee", frequencyDays: 90 });
    expect(form("Save changes")).toBeNull();
  });

  it("sends the last-in-touch date when the edit changed it", async () => {
    render(<PeoplePane view={view([MAYA, THEO])} />);
    await click(button("Edit Maya Chen"));
    fireEvent.change(screen.getByLabelText("Last in touch"), { target: { value: "2026-11-01" } });
    await click(button("Save changes"));
    expect(updatePerson).toHaveBeenCalledWith({
      id: 1,
      name: "Maya Chen",
      frequencyDays: 30,
      lastConnected: "2026-11-01",
    });
  });

  it("marks a person as done", async () => {
    render(<PeoplePane view={view([MAYA, THEO])} />);
    await doneToday("Theo Marsh");
    expect(markDone).toHaveBeenCalledWith(2);
  });

  it("after Done moves a person's row, focuses their Done button in its new place", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    button("Mark Maya Chen as done").focus();
    await doneToday("Maya Chen");
    // Done today: the list re-sorts and Maya moves to the end, which drops focus.
    const done = { ...MAYA, lastConnected: TODAY, due: false, nextTarget: "2026-12-19", notificationDate: "2026-12-14" };
    rerender(<PeoplePane view={view([THEO, LENA, done])} />);
    expect(document.activeElement).toBe(button("Mark Maya Chen as done"));
  });

  it("after Done on another day, focuses the Done button once the view shows that day", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    button("Mark Maya Chen as done").focus();
    await click(button("Mark Maya Chen as done"));
    const question = within(screen.getByRole("dialog"));
    await click(question.getByRole("button", { name: /^Another day/ }));
    fireEvent.change(question.getByLabelText("Another day"), { target: { value: "2026-11-14" } });
    await click(question.getByRole("button", { name: "Done" }));
    expect(markDone).toHaveBeenCalledWith(1);
    const done = { ...MAYA, lastConnected: "2026-11-14", due: false, nextTarget: "2026-12-19", notificationDate: "2026-12-14" };
    rerender(<PeoplePane view={view([THEO, LENA, done])} />);
    expect(document.activeElement).toBe(button("Mark Maya Chen as done"));
  });

  it("after Done, focuses the Done button when the new view arrives before the command returns", async () => {
    // The macOS app sends the new view first, while the command is still running.
    let finish!: (v: View) => void;
    markDone.mockReturnValue(new Promise<View>((resolve) => (finish = resolve)));
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    button("Mark Maya Chen as done").focus();
    await doneToday("Maya Chen");
    const after = view([THEO, LENA, { ...MAYA, lastConnected: TODAY, due: false }]);
    rerender(<PeoplePane view={after} />);
    act(() => (document.activeElement as HTMLElement | null)?.blur());
    expect(document.activeElement).toBe(document.body);
    await act(async () => finish(after));
    expect(document.activeElement).toBe(button("Mark Maya Chen as done"));
  });

  it("after a failed Done, doesn't take focus on a later view", async () => {
    markDone.mockRejectedValue({ kind: "storage", message: "Couldn't save the data file." });
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    button("Mark Maya Chen as done").focus();
    await doneToday("Maya Chen");
    act(() => button("Mark Maya Chen as done").blur());
    rerender(<PeoplePane view={view([THEO, { ...MAYA, lastConnected: TODAY }])} />);
    expect(document.activeElement).toBe(document.body);
  });

  it("after Done, a view from a new day uses up the focus target", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    button("Mark Maya Chen as done").focus();
    await doneToday("Maya Chen");
    act(() => button("Mark Maya Chen as done").blur());
    // Done was pressed just before midnight: Maya was in touch yesterday by the new view's today.
    rerender(<PeoplePane view={{ ...view([THEO, { ...MAYA, lastConnected: TODAY }]), today: "2026-11-19" }} />);
    expect(document.activeElement).toBe(button("Mark Maya Chen as done"));
    act(() => button("Mark Maya Chen as done").blur());
    rerender(<PeoplePane view={{ ...view([THEO, { ...MAYA, lastConnected: "2026-11-19" }]), today: "2026-11-19" }} />);
    expect(document.activeElement).toBe(document.body);
  });

  it("after Done, waits for the new view before focusing the Done button", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    button("Mark Maya Chen as done").focus();
    await doneToday("Maya Chen");
    // Focus is lost, then the old view comes again (a refresh that crossed the Done): wait.
    act(() => button("Mark Maya Chen as done").blur());
    rerender(<PeoplePane view={view([MAYA, THEO])} />);
    expect(document.activeElement).toBe(document.body);
    rerender(<PeoplePane view={view([THEO, { ...MAYA, lastConnected: TODAY }])} />);
    expect(document.activeElement).toBe(button("Mark Maya Chen as done"));
  });

  it("after Done on someone already in touch today, waits for a newer view", async () => {
    const today = { ...MAYA, lastConnected: TODAY };
    const shown = view([today, THEO]);
    const { rerender } = render(<PeoplePane view={shown} />);
    button("Mark Maya Chen as done").focus();
    await doneToday("Maya Chen");
    act(() => button("Mark Maya Chen as done").blur());
    // The same view again: not the new one yet.
    rerender(<PeoplePane view={shown} />);
    expect(document.activeElement).toBe(document.body);
    rerender(<PeoplePane view={view([THEO, today])} />);
    expect(document.activeElement).toBe(button("Mark Maya Chen as done"));
  });

  it("leaves focus alone after Done if the user moved it", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    await doneToday("Maya Chen");
    const edit = button("Edit Theo Marsh");
    edit.focus();
    rerender(<PeoplePane view={view([THEO, { ...MAYA, lastConnected: TODAY }])} />);
    expect(document.activeElement).toBe(edit);
  });

  it("shows a Done error in that person's row", async () => {
    markDone.mockRejectedValue({ kind: "storage", message: "Couldn't save the data file." });
    render(<PeoplePane view={view([MAYA, THEO])} />);
    await doneToday("Theo Marsh");
    const [maya, theo] = screen.getAllByTestId("person-row") as [HTMLElement, HTMLElement];
    expect(within(theo).getByRole("alert").textContent).toBe("Couldn't save the data file.");
    expect(within(maya).queryByRole("alert")).toBeNull();
  });

  it("backs out of a delete with Keep, and clears the selection with Deselect all", async () => {
    render(<PeoplePane view={view([MAYA, THEO])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    await click(button("Keep"));
    expect(screen.queryByText("Delete this person?")).toBeNull();
    expect(screen.getByText("1 selected")).toBeTruthy();
    await click(button("Deselect all"));
    expect(screen.queryByRole("region", { name: "Selected people" })).toBeNull();
    expect((screen.getByLabelText("Select Maya Chen") as HTMLInputElement).checked).toBe(false);
    expect(deletePeople).not.toHaveBeenCalled();
  });

  it("sends focus to the first selected person's checkbox after Deselect all", async () => {
    render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Lena Fischer"));
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    const deselect = button("Deselect all");
    act(() => deselect.focus());
    await click(deselect);
    expect(screen.queryByRole("region", { name: "Selected people" })).toBeNull();
    // Theo comes before Lena in the list.
    expect(document.activeElement).toBe(screen.getByLabelText("Select Theo Marsh"));
  });

  it("leaves focus alone after Deselect all when the button didn't have it", async () => {
    render(<PeoplePane view={view([MAYA, THEO])} />);
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    // A mouse click in WebKit doesn't focus the button.
    act(() => (document.activeElement as HTMLElement | null)?.blur());
    await click(button("Deselect all"));
    expect(screen.queryByRole("region", { name: "Selected people" })).toBeNull();
    expect(document.activeElement).toBe(document.body);
  });

  it("deletes the selected people after a confirmation", async () => {
    render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Lena Fischer"));
    expect(screen.getByText("2 selected")).toBeTruthy();
    await click(button("Delete selected…"));
    expect(screen.getByText("Delete these people?")).toBeTruthy();
    await click(button("Delete"));
    expect(deletePeople).toHaveBeenCalledWith([1, 3]);
    expect(screen.queryByRole("region", { name: "Selected people" })).toBeNull();
  });

  it("shows a delete error in the selection bar", async () => {
    deletePeople.mockRejectedValue({ kind: "storage", message: "Couldn't save the data file." });
    render(<PeoplePane view={view([MAYA, THEO])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    expect(screen.getByText("Delete this person?")).toBeTruthy();
    await click(button("Delete"));
    const bar = screen.getByRole("region", { name: "Selected people" });
    expect(within(bar).getByRole("alert").textContent).toBe("Couldn't save the data file.");
    expect(within(bar).getByText("1 selected")).toBeTruthy();
  });

  it("stops counting a selected person who is gone", () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    expect(screen.getByText("2 selected")).toBeTruthy();
    // Maya is deleted from another window.
    rerender(<PeoplePane view={view([THEO])} />);
    expect(screen.getByText("1 selected")).toBeTruthy();
    rerender(<PeoplePane view={view([])} />);
    expect(screen.queryByRole("region", { name: "Selected people" })).toBeNull();
  });

  it("keeps another person's edit form open when deleting", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    await click(button("Edit Maya Chen"));
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Maya Chen-Ito" } });
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    await click(button("Delete selected…"));
    await click(button("Delete"));
    expect(deletePeople).toHaveBeenCalledWith([2]);
    // The new view, without Theo, arrives.
    rerender(<PeoplePane view={view([MAYA])} />);
    expect(form("Save changes")).toBeTruthy();
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("Maya Chen-Ito");
  });

  it("after a delete, focuses the checkbox of the person now in the first deleted one's place", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    await click(button("Delete"));
    // The bar is gone, and the new view hasn't arrived yet.
    expect(document.activeElement).toBe(document.body);
    rerender(<PeoplePane view={view([LENA])} />);
    expect(document.activeElement).toBe(screen.getByLabelText("Select Lena Fischer"));
  });

  it("after deleting the last people in the list, focuses the one before them", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Lena Fischer"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    await click(button("Delete"));
    rerender(<PeoplePane view={view([MAYA, THEO])} />);
    expect(document.activeElement).toBe(screen.getByLabelText("Select Theo Marsh"));
  });

  it("after deleting everyone, focuses Add person", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    await click(button("Delete"));
    rerender(<PeoplePane view={view([])} />);
    expect(document.activeElement).toBe(button("Add person"));
  });

  it("after a delete, focuses the person in the deleted one's place when the new view came first", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    let finish!: (v: View) => void;
    deletePeople.mockReturnValue(new Promise((resolve) => (finish = resolve)));
    await click(button("Delete"));
    // The new view, without Theo, arrives before the command returns.
    rerender(<PeoplePane view={view([MAYA, LENA])} />);
    await act(async () => finish(view([MAYA, LENA])));
    expect(document.activeElement).toBe(screen.getByLabelText("Select Lena Fischer"));
  });

  it("after a delete, focuses the person who took the deleted one's place even if the list was reordered", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    await click(button("Delete"));
    rerender(<PeoplePane view={view([LENA, THEO])} />);
    expect(document.activeElement).toBe(screen.getByLabelText("Select Theo Marsh"));
  });

  it("after deleting everyone while adding someone, focuses the add form", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA])} />);
    await click(button("Add person"));
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    await click(button("Delete"));
    rerender(<PeoplePane view={view([])} />);
    expect(document.activeElement).toBe(screen.getByLabelText("Name"));
  });

  it("leaves focus alone after a delete if the user moved it", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    button("Delete").focus();
    await click(button("Delete"));
    const edit = button("Edit Theo Marsh");
    edit.focus();
    rerender(<PeoplePane view={view([THEO])} />);
    expect(document.activeElement).toBe(edit);
  });

  it("closes the edit form when its person is deleted", async () => {
    render(<PeoplePane view={view([MAYA, THEO])} />);
    await click(button("Edit Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    await click(button("Delete"));
    expect(form("Save changes")).toBeNull();
  });
});
