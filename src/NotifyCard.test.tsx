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

  it("goes away on Not now without asking", () => {
    renderCard(view([someone]), notify());
    fireEvent.click(screen.getByRole("button", { name: "Not now" }));
    expect(title()).toBeNull();
    expect(allow).not.toHaveBeenCalled();
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
