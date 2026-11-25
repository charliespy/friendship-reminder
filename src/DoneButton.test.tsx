import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { IsoDate, Person, View } from "./api";

const markDone = vi.fn<(id: number, on?: IsoDate) => Promise<View>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: { markDone: (id: number, on?: IsoDate) => markDone(id, on) },
}));

const { DoneButton } = await import("./DoneButton");

const TODAY = "2026-11-25"; // a Wednesday

function person(p: Partial<Person> = {}): Person {
  return {
    id: 7,
    name: "Maya Chen",
    frequencyDays: 30,
    lastConnected: "2026-10-20",
    nextTarget: "2026-11-28",
    notificationDate: "2026-11-23",
    overdue: false,
    due: true,
    suggestedDay: null,
    earliestDone: "2026-10-20",
    ...p,
  };
}

async function click(el: HTMLElement) {
  await act(async () => {
    fireEvent.click(el);
  });
}

async function ask(p: Person) {
  render(<DoneButton person={p} today={TODAY} />);
  await click(screen.getByRole("button", { name: "Mark Maya Chen as done" }));
  return within(screen.getByRole("dialog", { name: "When were you in touch with Maya Chen?" }));
}

const names = (d: ReturnType<typeof within>) =>
  d.getAllByRole("button").map((b: HTMLElement) => b.textContent);

describe("DoneButton", () => {
  beforeEach(() => {
    markDone.mockReset();
    markDone.mockResolvedValue({} as View);
  });
  afterEach(cleanup);

  it("asks first, and does nothing until answered", async () => {
    expect(screen.queryByRole("dialog")).toBeNull();
    const d = await ask(person());
    expect(markDone).not.toHaveBeenCalled();
    // The target is still ahead: today or another day.
    expect(names(d)).toEqual(["TodayWed, Nov 25", "Another day…Pick it on a calendar.", "Cancel"]);
  });

  it("marks today when answered Today", async () => {
    const d = await ask(person());
    await click(d.getByRole("button", { name: /^Today/ }));
    expect(markDone).toHaveBeenCalledWith(7, undefined);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("offers the day suggested when it has gone by", async () => {
    const d = await ask(person({ overdue: true, suggestedDay: "2026-11-21" }));
    expect(names(d)[0]).toBe("On the day suggestedSat, Nov 21");
    await click(d.getByRole("button", { name: /^On the day suggested/ }));
    expect(markDone).toHaveBeenCalledWith(7, "2026-11-21");
  });

  it("says so when today is the day suggested, without a second answer for it", async () => {
    const d = await ask(person({ nextTarget: TODAY, suggestedDay: TODAY }));
    expect(names(d)).toEqual([
      "TodayWed, Nov 25 · the day suggested",
      "Another day…Pick it on a calendar.",
      "Cancel",
    ]);
  });

  it("takes another day from the calendar, no later than today", async () => {
    const d = await ask(person());
    await click(d.getByRole("button", { name: /^Another day/ }));
    const field = d.getByLabelText("Another day") as HTMLInputElement;
    expect(field.max).toBe(TODAY);
    expect(field.min).toBe("2026-10-20");
    fireEvent.change(field, { target: { value: "2026-11-22" } });
    await click(d.getByRole("button", { name: "Done" }));
    expect(markDone).toHaveBeenCalledWith(7, "2026-11-22");
  });

  it("closes on Cancel without changing anything", async () => {
    const d = await ask(person());
    await click(d.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(markDone).not.toHaveBeenCalled();
  });

  it("stays open with the reason when it fails", async () => {
    markDone.mockRejectedValue({ kind: "invalid", message: "Pick that day or later." });
    const d = await ask(person());
    await click(d.getByRole("button", { name: /^Today/ }));
    expect(d.getByRole("alert").textContent).toBe("Pick that day or later.");
  });

  it("forgets an old error when asked again", async () => {
    markDone.mockRejectedValue({ kind: "invalid", message: "Pick that day or later." });
    const d = await ask(person());
    await click(d.getByRole("button", { name: /^Today/ }));
    await click(d.getByRole("button", { name: "Cancel" }));
    await click(screen.getByRole("button", { name: "Mark Maya Chen as done" }));
    expect(within(screen.getByRole("dialog")).queryByRole("alert")).toBeNull();
  });

  it("offers days from the earliest one the app takes", async () => {
    const d = await ask(person({ lastConnected: "2026-11-26", earliestDone: TODAY }));
    await click(d.getByRole("button", { name: /^Another day/ }));
    expect((d.getByLabelText("Another day") as HTMLInputElement).min).toBe(TODAY);
  });
});
