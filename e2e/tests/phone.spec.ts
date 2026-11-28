import { devices, expect, test } from "@playwright/test";
import { SAMPLE, answerDone, seed, setNotificationAccess } from "./helpers";

// The iPhone app's page, at an iPhone's size.
test.use({ viewport: devices["iPhone 15 Pro"].viewport, hasTouch: true });

test("Today lists who's due, and Done there moves them on", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  await page.goto("/phone.html");
  await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Today, 3 due" })).toBeVisible();
  await expect(page.getByTestId("due-person")).toHaveCount(3);
  await page.getByRole("button", { name: "Mark Maya Chen as done" }).click();
  await answerDone(page, "Today");
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

test("long names in People are cut short and keep their Due tag", async ({ page, request }) => {
  const names = ["Maya Chen", "Christopher Montgomery", "Alexandra Papadopoulos", "Oluwaseun Adebayo-Williams"];
  await seed(request, { people: names.map((name) => ({ name, frequencyDays: 30, lastConnected: "2026-10-10" })) });
  await page.goto("/phone.html");
  await page.getByRole("tab", { name: "People" }).click();
  const rows = page.getByTestId("person-row");
  await expect(rows).toHaveCount(names.length);
  for (const row of await rows.all()) {
    const name = row.locator(".person-name");
    const tag = name.locator(".tag");
    await expect(tag).toHaveText("Due");
    const nameBox = (await name.boundingBox())!;
    const tagBox = (await tag.boundingBox())!;
    // The whole tag lies inside the name's box, which cuts off whatever overflows it.
    expect(tagBox.x).toBeGreaterThanOrEqual(nameBox.x);
    expect(tagBox.x + tagBox.width).toBeLessThanOrEqual(nameBox.x + nameBox.width + 0.5);
  }
});

test("Settings has no Open at login on the phone", async ({ page, request }) => {
  await seed(request);
  await page.goto("/phone.html");
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.getByText("Send a notification")).toBeVisible();
  await expect(page.getByText("Open at login")).toHaveCount(0);
});

test("iOS is asked about notifications only when reminders are turned on, not at launch", async ({ page, request }) => {
  // Nobody in the list yet: nothing to remind about, so no card.
  await seed(request);
  await setNotificationAccess(request, "notAsked");
  await page.goto("/phone.html");
  await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Get reminded on the day" })).toHaveCount(0);

  // With people, Today says what reminders are; "Not now" hides it until the app opens again.
  await seed(request, { people: SAMPLE });
  await setNotificationAccess(request, "notAsked");
  await page.reload();
  const card = page.getByRole("region", { name: "Get reminded on the day" });
  await expect(card).toBeVisible();
  // From the keyboard: the button goes away with the card, and focus goes to Today's heading.
  await card.getByRole("button", { name: "Not now" }).focus();
  await page.keyboard.press("Enter");
  await expect(card).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Today" })).toBeFocused();
  // Settings still offers it.
  await page.getByRole("tab", { name: "Settings" }).click();
  await page.getByRole("button", { name: "Allow notifications" }).click();
  await expect(page.getByRole("button", { name: "Allow notifications" })).toHaveCount(0);

  // Next launch: allowed, so no card.
  await page.reload();
  await expect(page.getByRole("heading", { name: "Today" })).toBeVisible();
  await expect(card).toHaveCount(0);

  // "Turn on reminders" on Today asks.
  await setNotificationAccess(request, "notAsked");
  await page.reload();
  await card.getByRole("button", { name: "Turn on reminders" }).focus();
  await page.keyboard.press("Enter");
  await expect(card).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Today" })).toBeFocused();

  // Refused in iOS: Settings says where to turn them on.
  await setNotificationAccess(request, "denied");
  await page.reload();
  await expect(card).toHaveCount(0);
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.getByText("Notifications are off for Friendship Reminder.", { exact: false })).toBeVisible();
});
