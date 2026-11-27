import { expect, test, type Route } from "@playwright/test";
import { openPreferences, seed, setToday } from "./helpers";

const ANA = { name: "Ana Torres", frequencyDays: 30 as const, lastConnected: "2026-11-18" };

test("switching to weekdays reschedules everyone", async ({ page, request }) => {
  await seed(request, { people: [ANA] });
  await openPreferences(page, "Settings");
  await expect(page.getByLabel(/Weekends only/)).toBeChecked();
  // Controlled inputs change only after the round trip to the app, so click and then wait.
  await page.getByLabel(/Weekdays only/).click();
  await expect(page.getByLabel(/Weekdays only/)).toBeChecked();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("status")).toHaveText("Saved.");
  await page.getByRole("tab", { name: "People" }).click();
  // Nov 18 + 30 = Fri Dec 18, a weekday: aimed for and reminded that day.
  await expect(page.getByTestId("person-row")).toContainText("Reminder in 4 weeks, Fri, Dec 18");
});

test("Custom day aims only for the days picked", async ({ page, request }) => {
  await seed(request, { people: [ANA] });
  await openPreferences(page, "Settings");
  await page.getByLabel(/Custom day/).click();
  await expect(page.getByLabel(/Custom day/)).toBeChecked();
  await page.getByRole("button", { name: "Save" }).click();
  const days = page.getByRole("group", { name: "Days you're free" });
  // All seven at first: Nov 18 + 30 = Fri Dec 18 stays.
  await expect(days.getByRole("button", { pressed: true })).toHaveCount(7);
  await page.getByRole("tab", { name: "People" }).click();
  await expect(page.getByTestId("person-row")).toContainText("Reminder in 4 weeks, Fri, Dec 18");
  // Not free on Fridays: Saturday Dec 19 instead, with the Monday heads-up.
  await page.getByRole("tab", { name: "Settings" }).click();
  await days.getByRole("button", { name: "Friday" }).click();
  await expect(days.getByRole("button", { name: "Friday" })).toHaveAttribute("aria-pressed", "false");
  // Not until Save.
  await page.getByRole("tab", { name: "People" }).click();
  await expect(page.getByTestId("person-row")).toContainText("Reminder in 4 weeks, Fri, Dec 18");
  await page.getByRole("tab", { name: "Settings" }).click();
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("status")).toHaveText("Saved.");
  await page.getByRole("tab", { name: "People" }).click();
  await expect(page.getByTestId("person-row")).toContainText("Reminder in 4 weeks, Mon, Dec 14");
});

test("notifications and open at login can be switched off and on", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page, "Settings");
  const notify = page.getByLabel(/Send a notification/);
  const login = page.getByLabel(/Open at login/);
  await expect(notify).toBeChecked();
  await expect(login).not.toBeChecked();
  await notify.click();
  await expect(notify).not.toBeChecked();
  await login.click();
  await expect(login).toBeChecked();
  await page.reload();
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(notify).not.toBeChecked();
  await expect(login).toBeChecked();
});

test("a weekend that goes by without Done pushes the person to the next one", async ({ page, request }) => {
  await seed(request, { people: [{ name: "Maya Chen", frequencyDays: 30, lastConnected: "2026-11-01" }] });
  // Nov 1 + 30 = Tue Dec 1 → Sat Dec 5, reminded Mon Nov 30.
  await setToday(request, "2026-12-01");
  await page.goto("/");
  await expect(page.getByTestId("due-person")).toContainText("Aim for Sat, Dec 5");
  // The weekend goes by.
  await setToday(request, "2026-12-07");
  await page.reload();
  await expect(page.getByTestId("due-person")).toContainText("Missed last time · aim for Sat, Dec 12");
});

test("signing in with Apple shows the account, and signing out clears it", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page, "Settings");
  // The test server stands in for Apple's sheet and the account server.
  await page.getByRole("button", { name: /Sign in with Apple/ }).click();
  await expect(page.getByText("Signed in with Apple")).toBeVisible();
  await expect(page.getByTestId("account-email")).toHaveText("ana.torres@example.com");
  await page.reload();
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.getByText("Signed in with Apple")).toBeVisible();
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByRole("button", { name: /Sign in with Apple/ })).toBeEnabled();
});

test("when the account's buttons swap, focus goes to the one that took the pressed one's place", async ({
  page,
  request,
}) => {
  // The sign-in waits here until Cancel answers it, so the waiting row can be checked.
  let waiting: Route | null = null;
  await page.route("**/__friends/invoke", async (route) => {
    const { cmd } = route.request().postDataJSON() as { cmd: string };
    if (cmd === "sign_in_with_apple" && !waiting) waiting = route;
    else if (cmd === "cancel_sign_in") {
      // As in the app: Cancel wakes the waiting sign-in, which answers with the account unchanged.
      const answer = await route.fetch();
      await route.fulfill({ response: answer });
      await waiting?.fulfill({ response: answer });
    } else await route.continue();
  });
  const press = async (name: string | RegExp) => {
    await page.getByRole("button", { name }).focus();
    await page.keyboard.press("Enter");
  };
  await seed(request, { people: [ANA] });
  await openPreferences(page, "Settings");

  // Sign in with Apple gives way to Cancel while it waits, and Cancel back to Sign in with Apple.
  await press(/Sign in with Apple/);
  await expect(page.getByRole("button", { name: "Cancel" })).toBeFocused();
  await expect.poll(() => waiting !== null).toBe(true);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: /Sign in with Apple/ })).toBeFocused();

  // A finished sign-in gives way to Sign out, and Sign out back to Sign in with Apple.
  await page.keyboard.press("Enter");
  await expect(page.getByText("Signed in with Apple")).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: /Sign in with Apple/ })).toBeFocused();
});
