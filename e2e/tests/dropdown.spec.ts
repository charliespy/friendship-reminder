import { expect, test } from "@playwright/test";
import { SAMPLE, answerDone, seed } from "./helpers";

test("lists who to reach out to now and who's coming up", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Friendship Reminder" })).toBeVisible();
  await expect(page.getByText("Wed, Nov 18")).toBeVisible();
  const due = page.getByTestId("due-person");
  await expect(due).toHaveCount(3);
  await expect(due.first()).toContainText("Missed last time · aim for Sat, Nov 21");
  await expect(page.getByTestId("upcoming-person")).toHaveCount(3);
  await expect(page.getByTestId("upcoming-person").first()).toContainText("Sam Whitfield");
});

test("Done moves a person from now to later", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  await page.goto("/");
  await page.getByRole("button", { name: "Mark Maya Chen as done" }).click();
  await answerDone(page, "Today");
  await expect(page.getByTestId("due-person")).toHaveCount(2);
  await expect(page.getByTestId("due-person").filter({ hasText: "Maya Chen" })).toHaveCount(0);
  // Done today (Wed Nov 18) + 30 days = Fri Dec 18 → Sat Dec 19, reminded Mon Dec 14. Sam is also on Dec 14,
  // and the list goes by reminder day, then name: Maya before Sam.
  const upcoming = page.getByTestId("upcoming-person");
  await expect(upcoming.first()).toContainText("Maya Chen");
  await expect(upcoming.first()).toContainText("Mon, Dec 14");
});

test("an empty list points to Preferences", async ({ page, request }) => {
  await seed(request);
  await page.goto("/");
  await expect(page.getByText("Nobody here yet")).toBeVisible();
  await expect(page.getByRole("button", { name: "Preferences…" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Quit Friendship Reminder" })).toBeVisible();
});

test("everyone done says all caught up", async ({ page, request }) => {
  await seed(request, { people: [{ name: "Lena Fischer", frequencyDays: 365, lastConnected: "2026-03-02" }] });
  await page.goto("/");
  await expect(page.getByText("You're all caught up.")).toBeVisible();
});

test("Done sends focus to the next person to reach out to, then to the caught-up message", async ({ page, request }) => {
  await seed(request, { people: SAMPLE.slice(0, 2) });
  await page.goto("/");
  await expect(page.getByTestId("due-person")).toHaveCount(2);
  await page.getByRole("button", { name: "Mark Maya Chen as done" }).focus();
  await page.keyboard.press("Enter");
  // The question opens with focus on its first answer.
  await expect(page.getByRole("dialog").getByRole("button", { name: /^On the day suggested/ })).toBeFocused();
  const today = () => page.getByRole("dialog").getByRole("button", { name: /^Today/ });
  await today().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("due-person")).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Mark Jonah Okafor as done" })).toBeFocused();
  await page.keyboard.press("Enter");
  await today().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByText("You're all caught up.")).toBeFocused();
});
