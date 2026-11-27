import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";
import { NotifyCard } from "./NotifyCard";
import type { NotificationAccessState } from "./useNotificationAccess";

function view(people: Person[], notifications = true): View {
  return {
    today: "2026-11-18",
    people,
    blocked: [],
    settings: { dayToggle: "weekendsOnly", customDays: ["sat", "sun"], notifications },
    unreadableFile: null,
    guide: { signIn: true, import: true },
  };
}

const someone = { id: 1, name: "Maya Chen" } as Person;
const allow = vi.fn<() => Promise<void>>();

function notify(more: Partial<NotificationAccessState> = {}): NotificationAccessState {
  return { access: "notAsked", allow, busy: false, error: null, ...more };
}

function renderCard(v: View, n: NotificationAccessState) {
  return render(<NotifyCard view={v} notify={n} afterClose={() => null} />);
}

const title = () => screen.queryByRole("heading", { name: "Get reminded on the day" });

afterEach(() => {
  cleanup();
  allow.mockReset();
});

describe("NotifyCard", () => {
  it("shows when iOS hasn't been asked, reminders are on, and the list has people", () => {
    renderCard(view([someone]), notify());
    expect(title()).toBeTruthy();
  });

  it.each([
    ["access is allowed", view([someone]), notify({ access: "allowed" })],
    ["access is denied", view([someone]), notify({ access: "denied" })],
    ["access couldn't be read", view([someone]), notify({ access: "unknown" })],
    ["access isn't read yet", view([someone]), notify({ access: null })],
    ["reminders are off in Settings", view([someone], false), notify()],
    ["the list is empty", view([]), notify()],
  ])("stays hidden when %s", (_, v, n) => {
    renderCard(v, n);
    expect(title()).toBeNull();
  });

  it("asks iOS on Turn on reminders", () => {
    renderCard(view([someone]), notify());
    fireEvent.click(screen.getByRole("button", { name: "Turn on reminders" }));
    expect(allow).toHaveBeenCalledTimes(1);
  });

  it("goes away on Not now without asking, until the app opens again", () => {
    const { rerender, unmount } = renderCard(view([someone]), notify());
    fireEvent.click(screen.getByRole("button", { name: "Not now" }));
    expect(title()).toBeNull();
    expect(allow).not.toHaveBeenCalled();

    // A new view or a fresh read of access doesn't bring it back.
    rerender(<NotifyCard view={view([someone, { ...someone, id: 2 }])} notify={notify()} afterClose={() => null} />);
    expect(title()).toBeNull();

    // A card drawn afresh (as when the app opens again) shows again.
    unmount();
    renderCard(view([someone]), notify());
    expect(title()).toBeTruthy();
  });

  it("hands focus to what's next when Not now takes it away", () => {
    const next = document.createElement("h2");
    next.tabIndex = -1;
    document.body.append(next);
    render(<NotifyCard view={view([someone])} notify={notify()} afterClose={() => next} />);
    const notNow = screen.getByRole("button", { name: "Not now" });
    notNow.focus();
    fireEvent.click(notNow);
    expect(title()).toBeNull();
    expect(document.activeElement).toBe(next);
    next.remove();
  });

  it("shows what went wrong asking", () => {
    renderCard(view([someone]), notify({ error: "Couldn't ask iOS." }));
    expect(screen.getByRole("alert").textContent).toBe("Couldn't ask iOS.");
  });

  it("turns its buttons off while iOS is asking", () => {
    renderCard(view([someone]), notify({ busy: true }));
    for (const name of ["Not now", "Turn on reminders"]) {
      expect((screen.getByRole("button", { name }) as HTMLButtonElement).disabled).toBe(true);
    }
  });
});
