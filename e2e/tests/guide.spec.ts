import { expect, test } from "@playwright/test";
import { SAMPLE, openCard, openPreferences, seed, seedNewUser, setContactsAccess } from "./helpers";

test("a new user signs in or not, then picks people from Contacts", async ({ page, request }) => {
  await seedNewUser(request);
  await openPreferences(page);
  await expect(page.getByRole("heading", { name: "Welcome to Friendship Reminder" })).toBeVisible();
  await expect(page.getByRole("tab")).toHaveCount(0);
  await page.getByRole("button", { name: "Not now" }).click();

  await expect(page.getByRole("heading", { name: "Who do you want to keep up with?" })).toBeVisible();
  // The app says why it wants Contacts before the system asks.
  await expect(page.getByText("Add people from your Contacts")).toBeVisible();
  await expect(page.getByLabel("Search contacts")).toHaveCount(0);
  await page.getByRole("button", { name: "Continue" }).click();
  const add = page.getByRole("button", { name: /^Add/ });
  await expect(add).toBeDisabled();
  await page.getByLabel("Search contacts").fill("chloe");
  await page.getByRole("checkbox", { name: "Chloé Martin" }).check();
  await page.getByLabel("Search contacts").fill("");
  await page.getByRole("checkbox", { name: "Kofi Mensah" }).check();
  // Each person ticked gets their own "how often", monthly to start with.
  await expect(page.getByLabel("How often for Chloé Martin")).toHaveValue("30");
  await page.getByLabel("How often for Kofi Mensah").selectOption({ label: "Quarterly" });
  await expect(add).toHaveText("Add 2 people");
  await add.click();

  const rows = page.getByTestId("person-row");
  await expect(rows).toHaveCount(2);
  // In touch today (Wed Nov 18) + 30 = Fri Dec 18 → Sat Dec 19; reminder Mon Dec 14.
  await expect(rows.filter({ hasText: "Chloé Martin" })).toContainText("Monthly · Reminder in 4 weeks, Mon, Dec 14");
  // + 90 = Tue Feb 16 → Sat Feb 20; reminder Mon Feb 15.
  await expect(rows.filter({ hasText: "Kofi Mensah" })).toContainText("Quarterly · Reminder in 3 months, Mon, Feb 15, 2027");

  // Done once: the guide doesn't come back.
  await page.reload();
  await expect(page.getByRole("tab", { name: /People/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Welcome to Friendship Reminder" })).toHaveCount(0);
});

test("signing in on the first page leads to Contacts, which can be skipped", async ({ page, request }) => {
  await seedNewUser(request);
  await openPreferences(page);
  await page.getByRole("button", { name: "Sign in with Apple" }).click();
  await expect(page.getByRole("heading", { name: "Who do you want to keep up with?" })).toBeVisible();
  await page.getByRole("button", { name: "Skip" }).click();
  await expect(page.getByText("Nobody yet")).toBeVisible();
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.getByText("Signed in with Apple")).toBeVisible();
});

test("someone who already uses the app never sees the guide", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page);
  await expect(page.getByRole("tab", { name: /People/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Welcome to Friendship Reminder" })).toHaveCount(0);
});

test("Add person offers people from Contacts, and marks who's in the list already", async ({ page, request }) => {
  await seed(request, { people: [{ name: "Ana Torres", frequencyDays: 30, lastConnected: "2026-11-01" }] });
  // Contacts were shared on an earlier import.
  await setContactsAccess(request, "allowed");
  await openPreferences(page);
  await page.getByRole("button", { name: "Add person" }).click();
  await page.getByRole("button", { name: "Import from Contacts…" }).click();
  const picker = page.getByRole("region", { name: "Contacts" });
  await expect(picker.getByRole("checkbox", { name: "Ana Torres, already added" })).toBeDisabled();
  await expect(picker.getByText("Already added")).toBeVisible();
  await picker.getByRole("checkbox", { name: "Ben Okafor" }).check();
  await page.getByRole("button", { name: "Add 1 person" }).click();
  await expect(page.getByTestId("person-row")).toHaveCount(2);
  await expect(page.getByRole("region", { name: "Contacts" })).toHaveCount(0);

  // Ben's card now carries the link back, so he shows as added too.
  await page.getByRole("button", { name: "Add person" }).click();
  await page.getByRole("button", { name: "Import from Contacts…" }).click();
  await expect(picker.getByRole("checkbox", { name: "Ben Okafor, already added" })).toBeDisabled();
});

test("without access to Contacts the picker says how to allow it, and Cancel goes back", async ({ page, request }) => {
  await seed(request);
  await setContactsAccess(request, "denied");
  await openPreferences(page);
  await page.getByRole("button", { name: "Add person" }).click();
  await page.getByRole("button", { name: "Import from Contacts…" }).click();
  await expect(page.getByText("Friendship Reminder can't see your contacts")).toBeVisible();
  await expect(page.getByText(/Privacy & Security › Contacts/)).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByRole("form", { name: "Add person" })).toBeVisible();
});

test("on iPhone, people shared in Apple's sheet go straight to how often", async ({ page, request }) => {
  await seedNewUser(request);
  await setContactsAccess(request, "notAskedLimited");
  await page.goto("/phone.html");
  await page.getByRole("button", { name: "Not now" }).click();
  await expect(page.getByText(/choose just the people you want/)).toBeVisible();
  await page.getByRole("button", { name: "Continue" }).click();

  // Ben, Elif and Iris were chosen in Apple's sheet: no list to tick again, just how often.
  const picker = page.getByRole("region", { name: "Contacts" });
  await expect(picker.getByRole("heading", { name: "How often?" })).toBeVisible();
  await expect(picker.getByLabel("Search contacts")).toHaveCount(0);
  await expect(picker.getByRole("checkbox")).toHaveCount(3);
  await expect(picker.getByRole("checkbox", { name: "Elif Yılmaz" })).toBeChecked();
  await picker.getByLabel("How often for Elif Yılmaz").selectOption({ label: "Quarterly" });
  await page.getByRole("button", { name: "Add 3 people" }).click();

  await page.getByRole("tab", { name: /People/ }).click();
  const rows = page.getByTestId("person-row");
  await expect(rows).toHaveCount(3);
  await expect(rows.filter({ hasText: "Elif Yılmaz" })).toContainText("Quarterly");
  await expect(rows.filter({ hasText: "Ben Okafor" })).toContainText("Monthly");
});

test("with only some contacts shared, importing opens Apple's sheet to share more", async ({ page, request }) => {
  await seed(request, { people: SAMPLE.slice(0, 1) });
  await setContactsAccess(request, "limited");
  await page.goto("/phone.html");
  await page.getByRole("tab", { name: /People/ }).click();
  await page.getByRole("button", { name: "Add person" }).click();
  await page.getByRole("button", { name: "Import from Contacts…" }).click();

  // Apple's sheet shared Kofi and Lena: they go to how often.
  const picker = page.getByRole("region", { name: "Contacts" });
  await expect(picker.getByRole("heading", { name: "How often?" })).toBeVisible();
  await expect(picker.getByRole("checkbox")).toHaveCount(2);
  await picker.getByRole("checkbox", { name: "Lena Fischer" }).uncheck();
  await page.getByRole("button", { name: "Add 1 person" }).click();
  await expect(page.getByTestId("person-row").filter({ hasText: "Kofi Mensah" })).toHaveCount(1);
  await expect(page.getByTestId("person-row").filter({ hasText: "Lena Fischer" })).toHaveCount(0);

  // Nobody new shared this time: the people already shared, to tick, and a way to share more.
  await page.getByRole("button", { name: "Add person" }).click();
  await page.getByRole("button", { name: "Import from Contacts…" }).click();
  await expect(picker.getByLabel("Search contacts")).toBeVisible();
  await expect(picker.getByRole("checkbox", { name: "Kofi Mensah, already added" })).toBeDisabled();
  await expect(picker.getByRole("checkbox", { name: "Lena Fischer" })).not.toBeChecked();
  // Inside People on iPhone, each row still shows its name (not squeezed into the list's grid).
  await expect(picker.getByText("Lena Fischer")).toBeVisible();
  await expect(picker.getByRole("button", { name: "Choose more in Contacts…" })).toBeVisible();
});

test("on iPhone the guide fills the screen, without the tab bar", async ({ page, request }) => {
  await seedNewUser(request);
  await page.goto("/phone.html");
  await expect(page.getByRole("heading", { name: "Welcome to Friendship Reminder" })).toBeVisible();
  await expect(page.getByRole("tab")).toHaveCount(0);
  await page.getByRole("button", { name: "Not now" }).click();
  await page.getByRole("button", { name: "Skip" }).click();
  await expect(page.getByRole("tab", { name: /Today/ })).toBeVisible();
});

test("the link on a contact card opens the app at that person", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  // Contacts were shared on an earlier import.
  await setContactsAccess(request, "allowed");
  await openPreferences(page);
  await page.getByRole("button", { name: "Add person" }).click();
  await page.getByRole("button", { name: "Import from Contacts…" }).click();
  await page.getByRole("checkbox", { name: "Iris Novak" }).check();
  await page.getByRole("button", { name: "Add 1 person" }).click();
  const iris = page.getByTestId("person-row").filter({ hasText: "Iris Novak" });
  await expect(iris).toHaveCount(1);

  // A card without the app's link, then Iris's card (card-09), which got it when she was added.
  expect(await openCard(request, "card-01")).toBeNull();
  const id = await openCard(request, "card-09");
  expect(String(id)).toBe(await iris.getAttribute("data-person-id"));

  // The app comes forward on People, at Iris, lit up for a moment.
  await page.getByRole("tab", { name: "Settings" }).click();
  await page.reload();
  await expect(page.getByRole("tab", { name: /People/ })).toHaveAttribute("aria-selected", "true");
  await expect(iris).toHaveClass(/is-highlighted/);
  await expect(iris).not.toHaveClass(/is-highlighted/, { timeout: 5000 });
});
