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

test("a range that ends before it starts can't be added", async ({ page, request }) => {
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
