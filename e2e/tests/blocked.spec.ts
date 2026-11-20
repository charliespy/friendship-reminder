import { expect, test } from "@playwright/test";
import { openPreferences, seed } from "./helpers";

const ANA = { name: "Ana Torres", frequencyDays: 30 as const, lastConnected: "2026-11-18" };

test("blocking dates moves anyone due then, and removing them moves them back", async ({ page, request }) => {
  await seed(request, { people: [ANA] }); // target Sat Dec 19, reminder Mon Dec 14
  await openPreferences(page, "Blocked dates");
  await expect(page.getByText("No blocked dates")).toBeVisible();
  const form = page.getByRole("form", { name: "Add blocked dates" });
  await form.getByLabel("What").fill("Holidays in Lisbon");
  await form.getByLabel("From").fill("2026-12-18");
  await form.getByLabel("To").fill("2027-01-03");
  await form.getByRole("button", { name: "Block dates" }).click();
  const row = page.getByTestId("blocked-row");
  await expect(row).toContainText("Holidays in Lisbon");
  await expect(row).toContainText("Dec 18, 2026 – Jan 3, 2027");

  await page.getByRole("tab", { name: "People" }).click();
  // The first weekend after Jan 3 is Sat Jan 9; reminder Mon Jan 4.
  await expect(page.getByTestId("person-row")).toContainText("Mon, Jan 4, 2027");

  await page.getByRole("tab", { name: "Blocked dates" }).click();
  await row.getByRole("button", { name: /Remove/ }).click();
  await expect(page.getByText("No blocked dates")).toBeVisible();
  await page.getByRole("tab", { name: "People" }).click();
  await expect(page.getByTestId("person-row")).toContainText("Mon, Dec 14");
});

test("moving From past To moves To along", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page, "Blocked dates");
  const form = page.getByRole("form", { name: "Add blocked dates" });
  await form.getByLabel("From").fill("2026-12-10");
  // Moving "From" after "To" moves "To" along with it.
  await expect(form.getByLabel("To")).toHaveValue("2026-12-10");
});

test("dates more than ten years out are refused with a reason", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page, "Blocked dates");
  const form = page.getByRole("form", { name: "Add blocked dates" });
  await form.getByLabel("To").fill("2040-01-01");
  await form.getByRole("button", { name: "Block dates" }).click();
  await expect(form.getByRole("alert")).toHaveText("Pick dates from 1900 up to ten years from now.");
});

test("ranges in another year show their year", async ({ page, request }) => {
  await seed(request, {
    blocked: [
      { start: "2025-12-01", end: "2025-12-05", label: "Last year's trip" },
      { start: "2026-12-01", end: "2026-12-05", label: "Work trip" },
      { start: "2027-11-30", end: "2027-11-30", label: "One day next year" },
    ],
  });
  await openPreferences(page, "Blocked dates");
  const rows = page.getByTestId("blocked-row");
  await expect(rows.filter({ hasText: "Last year's trip" })).toContainText("Dec 1 – 5, 2025");
  await expect(rows.filter({ hasText: "Work trip" }).locator(".person-meta")).toHaveText("Dec 1 – 5");
  const nextYear = rows.filter({ hasText: "One day next year" });
  await expect(nextYear).toContainText("Nov 30, 2027");
  await expect(nextYear.getByRole("button", { name: "Remove One day next year Nov 30, 2027" })).toBeVisible();
});

test("Remove sends focus to the next range's Remove, then to the form", async ({ page, request }) => {
  await seed(request, {
    blocked: [
      { start: "2026-12-01", end: "2026-12-05", label: "Trip" },
      { start: "2027-01-10", end: "2027-01-12", label: "Visit" },
    ],
  });
  await openPreferences(page, "Blocked dates");
  const rows = page.getByTestId("blocked-row");
  await expect(rows).toHaveCount(2);
  await page.getByRole("button", { name: /^Remove Trip / }).focus();
  await page.keyboard.press("Enter");
  await expect(rows).toHaveCount(1);
  await expect(page.getByRole("button", { name: /^Remove Visit / })).toBeFocused();

  // The last range: focus goes to the form's first field.
  await page.keyboard.press("Enter");
  await expect(page.getByText("No blocked dates")).toBeVisible();
  await expect(page.getByRole("form", { name: "Add blocked dates" }).getByLabel("What")).toBeFocused();
});
