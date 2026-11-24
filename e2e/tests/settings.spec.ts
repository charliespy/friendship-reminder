import { expect, test } from "@playwright/test";
import { openPreferences, seed, setToday } from "./helpers";

const ANA = { name: "Ana Torres", frequencyDays: 30 as const, lastConnected: "2026-11-18" };

test("switching to weekdays reschedules everyone", async ({ page, request }) => {
  await seed(request, { people: [ANA] });
  await openPreferences(page, "Settings");
  await expect(page.getByLabel(/Weekends only/)).toBeChecked();
  // Controlled inputs change only after the round trip to the app, so click and then wait.
  await page.getByLabel(/Weekdays only/).click();
  await expect(page.getByLabel(/Weekdays only/)).toBeChecked();
  await page.getByRole("tab", { name: "People" }).click();
  // Nov 18 + 30 = Fri Dec 18, a weekday: aimed for and reminded that day.
  await expect(page.getByTestId("person-row")).toContainText("Reminder in 4 weeks, Fri, Dec 18");
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
