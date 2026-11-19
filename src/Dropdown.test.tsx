import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Person, View } from "./api";

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: { markDone: vi.fn(), openPreferences: vi.fn(), quit: vi.fn() },
}));

const { DropdownContent } = await import("./Dropdown");

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
});
