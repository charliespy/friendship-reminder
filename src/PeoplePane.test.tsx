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
    await click(button("Mark Theo Marsh as done"));
    expect(markDone).toHaveBeenCalledWith(2);
  });

  it("deletes the selected people after a confirmation", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO, LENA])} />);
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Lena Fischer"));
    expect(screen.getByText("2 selected")).toBeTruthy();
    await click(button("Delete selected…"));
    expect(screen.getByText("Delete these people?")).toBeTruthy();
    await click(button("Delete"));
    expect(deletePeople).toHaveBeenCalledWith([1, 3]);
    rerender(<PeoplePane view={view([THEO])} />);
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

  it("forgets the selection of a person who is gone", () => {
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
    deletePeople.mockResolvedValue(view([MAYA]));
    render(<PeoplePane view={view([MAYA, THEO])} />);
    await click(button("Edit Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Theo Marsh"));
    await click(button("Delete selected…"));
    await click(button("Delete"));
    expect(deletePeople).toHaveBeenCalledWith([2]);
    expect(form("Save changes")).toBeTruthy();
    expect((screen.getByLabelText("Name") as HTMLInputElement).value).toBe("Maya Chen");
  });

  it("closes the edit form when its person is deleted", async () => {
    const { rerender } = render(<PeoplePane view={view([MAYA, THEO])} />);
    await click(button("Edit Maya Chen"));
    fireEvent.click(screen.getByLabelText("Select Maya Chen"));
    await click(button("Delete selected…"));
    await click(button("Delete"));
    expect(form("Save changes")).toBeNull();
    // Maya coming back with the same id (an undo, say) doesn't reopen the form.
    rerender(<PeoplePane view={view([MAYA, THEO])} />);
    expect(form("Save changes")).toBeNull();
  });
});
