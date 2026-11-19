import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Blocked, View } from "./api";

const addBlocked = vi.fn<(b: unknown) => Promise<View>>();
const deleteBlocked = vi.fn<(id: number) => Promise<View>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    addBlocked: (b: unknown) => addBlocked(b),
    deleteBlocked: (id: number) => deleteBlocked(id),
  },
}));

const { BlockedPane } = await import("./BlockedPane");

function view(today: string, blocked: Blocked[] = []): View {
  return {
    today,
    people: [],
    blocked,
    settings: { dayToggle: "weekendsOnly", notifications: true },
    unreadableFile: null,
  };
}

const TRIP: Blocked = { id: 7, start: "2026-12-20", end: "2026-12-27", label: "Trip" };
const BUSY: Blocked = { id: 8, start: "2026-10-05", end: "2026-10-09", label: "" };

const input = (label: string) => screen.getByLabelText(label) as HTMLInputElement;
const blockDates = () => screen.getByRole("button", { name: "Block dates" });

async function submit() {
  await act(async () => {
    fireEvent.click(blockDates());
  });
}

afterEach(cleanup);

beforeEach(() => {
  for (const f of [addBlocked, deleteBlocked]) {
    f.mockReset();
    f.mockResolvedValue(view("2026-11-18"));
  }
});

describe("BlockedPane", () => {
  it("says when nothing is blocked", () => {
    render(<BlockedPane view={view("2026-11-18")} />);
    expect(screen.getByText("No blocked dates")).toBeTruthy();
  });

  it("lists blocked ranges, marks past ones as over, and removes one", async () => {
    render(<BlockedPane view={view("2026-11-18", [BUSY, TRIP])} />);
    const rows = screen.getAllByTestId("blocked-row");
    expect(rows).toHaveLength(2);
    const [busy, trip] = rows as [HTMLElement, HTMLElement];
    expect(within(busy).getByText("Blocked")).toBeTruthy();
    expect(busy.textContent).toContain(" · over");
    expect(within(trip).getByText("Trip")).toBeTruthy();
    expect(trip.textContent).not.toContain("over");
    await act(async () => {
      fireEvent.click(within(trip).getByRole("button", { name: /^Remove Trip / }));
    });
    expect(deleteBlocked).toHaveBeenCalledWith(7);
  });

  it("shows a remove error", async () => {
    deleteBlocked.mockRejectedValue({ kind: "storage", message: "Couldn't save the data file." });
    render(<BlockedPane view={view("2026-11-18", [TRIP])} />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Remove Trip / }));
    });
    expect(screen.getByRole("alert").textContent).toBe("Couldn't save the data file.");
  });

  it("starts both dates on today", () => {
    render(<BlockedPane view={view("2026-11-18")} />);
    expect(input("From").value).toBe("2026-11-18");
    expect(input("To").value).toBe("2026-11-18");
  });

  it("moves To forward when From is set later than it", () => {
    render(<BlockedPane view={view("2026-11-18")} />);
    fireEvent.change(input("From"), { target: { value: "2026-12-20" } });
    expect(input("To").value).toBe("2026-12-20");
    fireEvent.change(input("To"), { target: { value: "2026-12-27" } });
    // An earlier From leaves To where it is.
    fireEvent.change(input("From"), { target: { value: "2026-12-22" } });
    expect(input("From").value).toBe("2026-12-22");
    expect(input("To").value).toBe("2026-12-27");
  });

  it("keeps the dates on today until they are touched", () => {
    const { rerender } = render(<BlockedPane view={view("2026-11-18")} />);
    // The window stays open past midnight.
    rerender(<BlockedPane view={view("2026-11-19")} />);
    expect(input("From").value).toBe("2026-11-19");
    expect(input("To").value).toBe("2026-11-19");
    fireEvent.change(input("To"), { target: { value: "2026-11-25" } });
    rerender(<BlockedPane view={view("2026-11-20")} />);
    expect(input("From").value).toBe("2026-11-19");
    expect(input("To").value).toBe("2026-11-25");
  });

  it("adds a range with a trimmed label, then resets the form to follow today again", async () => {
    const { rerender } = render(<BlockedPane view={view("2026-11-18")} />);
    fireEvent.change(input("What"), { target: { value: "  Trip  " } });
    fireEvent.change(input("From"), { target: { value: "2026-12-20" } });
    fireEvent.change(input("To"), { target: { value: "2026-12-27" } });
    await submit();
    expect(addBlocked).toHaveBeenCalledWith({ start: "2026-12-20", end: "2026-12-27", label: "Trip" });
    expect(input("What").value).toBe("");
    expect(input("From").value).toBe("2026-11-18");
    expect(input("To").value).toBe("2026-11-18");
    rerender(<BlockedPane view={view("2026-11-19")} />);
    expect(input("From").value).toBe("2026-11-19");
    expect(input("To").value).toBe("2026-11-19");
  });

  it("shows an add error and keeps what was typed", async () => {
    addBlocked.mockRejectedValue({ kind: "invalid", message: "The range ends before it starts." });
    render(<BlockedPane view={view("2026-11-18")} />);
    fireEvent.change(input("What"), { target: { value: "Trip" } });
    fireEvent.change(input("From"), { target: { value: "2026-12-20" } });
    await submit();
    const form = screen.getByRole("form", { name: "Add blocked dates" });
    expect(within(form).getByRole("alert").textContent).toBe("The range ends before it starts.");
    expect(input("What").value).toBe("Trip");
    expect(input("From").value).toBe("2026-12-20");
  });

  it("disables Block dates while a date is empty", () => {
    render(<BlockedPane view={view("2026-11-18")} />);
    expect((blockDates() as HTMLButtonElement).disabled).toBe(false);
    fireEvent.change(input("To"), { target: { value: "" } });
    expect((blockDates() as HTMLButtonElement).disabled).toBe(true);
  });
});
