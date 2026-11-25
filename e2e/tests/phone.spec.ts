import { devices, expect, test } from "@playwright/test";
import { SAMPLE, seed } from "./helpers";

// The iPhone app's page, at an iPhone's size.
test.use({ viewport: devices["iPhone 15 Pro"].viewport, hasTouch: true });

test("Today lists who's due, and Done there moves them on", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  await page.goto("/phone.html");
  await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Today, 3 due" })).toBeVisible();
  await expect(page.getByTestId("due-person")).toHaveCount(3);
  await page.getByRole("button", { name: "Mark Maya Chen as done" }).click();
  await expect(page.getByTestId("due-person")).toHaveCount(2);
  await expect(page.getByRole("tab", { name: "Today, 2 due" })).toBeVisible();
});

test("a person added in People shows up in Today", async ({ page, request }) => {
  await seed(request);
  await page.goto("/phone.html");
  await expect(page.getByText("Add the people you want to keep up with in People.")).toBeVisible();
  await page.getByRole("tab", { name: "People" }).click();
  await page.getByRole("button", { name: "Add person" }).click();
  const form = page.getByRole("form", { name: "Add person" });
  await form.getByLabel("Name").fill("Ana Torres");
  await form.getByLabel("Last in touch").fill("2024-01-15");
  await form.getByRole("button", { name: "Add person" }).click();
  await expect(page.getByTestId("person-row")).toContainText("Ana Torres");
  await page.getByRole("tab", { name: /Today/ }).click();
  await expect(page.getByTestId("due-person")).toContainText("Missed last time");
  // The tab bar stays on screen, over the page.
  await expect(page.getByRole("tab", { name: "Settings" })).toBeInViewport();
});

test("Settings has no Open at login on the phone", async ({ page, request }) => {
  await seed(request);
  await page.goto("/phone.html");
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.getByText("Send a notification")).toBeVisible();
  await expect(page.getByText("Open at login")).toHaveCount(0);
});
