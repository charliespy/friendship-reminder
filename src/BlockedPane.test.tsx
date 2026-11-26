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
    settings: { dayToggle: "weekendsOnly", customDays: ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], notifications: true },
    unreadableFile: null,
    guide: { signIn: true, import: true },
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

  it("after Remove, focuses the Remove button of the range in its place, then the one before, then the form", async () => {
    const LATER: Blocked = { id: 9, start: "2027-01-10", end: "2027-01-12", label: "Visit" };
    const remove = (name: string) => screen.getByRole("button", { name: new RegExp(`^Remove ${name} `) });
    const pressRemove = async (name: string) => {
      remove(name).focus();
      await act(async () => {
        fireEvent.click(remove(name));
      });
    };
    const { rerender } = render(<BlockedPane view={view("2026-11-18", [BUSY, TRIP, LATER])} />);
    await pressRemove("Trip");
    // The new view hasn't arrived yet: wait for it.
    expect(document.activeElement).not.toBe(remove("Visit"));
    rerender(<BlockedPane view={view("2026-11-18", [BUSY, LATER])} />);
    expect(document.activeElement).toBe(remove("Visit"));

    // The last range: focus goes to the one before it.
    await pressRemove("Visit");
    rerender(<BlockedPane view={view("2026-11-18", [BUSY])} />);
    expect(document.activeElement).toBe(remove("blocked dates"));

    // No ranges left: focus goes to the form's first field.
    await pressRemove("blocked dates");
    rerender(<BlockedPane view={view("2026-11-18")} />);
    expect(document.activeElement).toBe(input("What"));
  });

  it("after Remove, focuses the next Remove button when the new view arrives before the command returns", async () => {
    // The macOS app sends the new view first, while the command is still running.
    let finish!: (v: View) => void;
    deleteBlocked.mockReturnValue(new Promise<View>((resolve) => (finish = resolve)));
    const remove = (name: string) => screen.getByRole("button", { name: new RegExp(`^Remove ${name} `) });
    const { rerender } = render(<BlockedPane view={view("2026-11-18", [BUSY, TRIP])} />);
    remove("Trip").focus();
    await act(async () => {
      fireEvent.click(remove("Trip"));
    });
    rerender(<BlockedPane view={view("2026-11-18", [BUSY])} />);
    // The Remove buttons are still disabled while the command runs.
    expect(document.activeElement).toBe(document.body);
    await act(async () => finish(view("2026-11-18", [BUSY])));
    expect(document.activeElement).toBe(remove("blocked dates"));
  });

  it("after a failed Remove, doesn't take focus on a later view", async () => {
    deleteBlocked.mockRejectedValue({ kind: "storage", message: "Couldn't save the data file." });
    const { rerender } = render(<BlockedPane view={view("2026-11-18", [BUSY, TRIP])} />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Remove Trip / }));
    });
    rerender(<BlockedPane view={view("2026-11-18", [BUSY])} />);
    expect(document.activeElement).toBe(document.body);
  });

  it("leaves focus alone after Remove if the user moved it", async () => {
    const { rerender } = render(<BlockedPane view={view("2026-11-18", [BUSY, TRIP])} />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Remove Trip / }));
    });
    input("From").focus();
    rerender(<BlockedPane view={view("2026-11-18", [BUSY])} />);
    expect(document.activeElement).toBe(input("From"));
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
    addBlocked.mockRejectedValue({ kind: "storage", message: "Couldn't save the data file." });
    render(<BlockedPane view={view("2026-11-18")} />);
    fireEvent.change(input("What"), { target: { value: "Trip" } });
    fireEvent.change(input("From"), { target: { value: "2026-12-20" } });
    await submit();
    const form = screen.getByRole("form", { name: "Add blocked dates" });
    expect(within(form).getByRole("alert").textContent).toBe("Couldn't save the data file.");
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
