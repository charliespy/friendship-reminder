import { expect, test } from "@playwright/test";
import { SAMPLE, TODAY, openPreferences, seed } from "./helpers";

test("add a person", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page);
  await expect(page.getByText("Nobody yet")).toBeVisible();
  await page.getByRole("button", { name: "Add person" }).click();
  const form = page.getByRole("form", { name: "Add person" });
  await expect(form.getByRole("button", { name: "Add person" })).toBeDisabled();
  await form.getByLabel("Name").fill("  Ana   Torres ");
  await form.getByLabel("How often").selectOption({ label: "Monthly" });
  await expect(form.getByLabel("Last in touch")).toHaveValue(TODAY);
  await form.getByRole("button", { name: "Add person" }).click();
  const row = page.getByTestId("person-row");
  await expect(row).toHaveCount(1);
  // Nov 18 + 30 = Fri Dec 18 → Sat Dec 19; reminder Monday Dec 14.
  await expect(row).toContainText("Ana Torres");
  await expect(row).toContainText("Monthly · Reminder in 4 weeks, Mon, Dec 14");
});

test("someone long out of touch is due now and marked missed", async ({ page, request }) => {
  await seed(request);
  await openPreferences(page);
  await page.getByRole("button", { name: "Add person" }).click();
  const form = page.getByRole("form", { name: "Add person" });
  await form.getByLabel("Name").fill("Old Friend");
  await form.getByLabel("Last in touch").fill("2024-01-15");
  await form.getByRole("button", { name: "Add person" }).click();
  await expect(page.getByTestId("person-row")).toContainText("Missed last time · aim for Sat, Nov 21");
  await expect(page.getByRole("tab", { name: /People/ })).toContainText("1");
});

test("edit a person's name and frequency", async ({ page, request }) => {
  await seed(request, { people: [{ name: "Theo Marsh", frequencyDays: 90, lastConnected: "2026-09-29" }] });
  await openPreferences(page);
  await page.getByRole("button", { name: "Edit Theo Marsh" }).click();
  const form = page.getByRole("form", { name: "Save changes" });
  await form.getByLabel("Name").fill("Theodore Marsh");
  await form.getByLabel("How often").selectOption({ label: "Yearly" });
  await form.getByRole("button", { name: "Save changes" }).click();
  await expect(form).toHaveCount(0);
  // Sep 29, 2026 + 365 = Wed Sep 29, 2027 → Sat Oct 2; reminder Mon Sep 27, 2027.
  await expect(page.getByTestId("person-row")).toContainText("Theodore Marsh");
  await expect(page.getByTestId("person-row")).toContainText("Yearly · Reminder in 10 months, Mon, Sep 27, 2027");
});

test("an edit that can't be saved shows why and keeps the form", async ({ page, request }) => {
  await seed(request, { people: [{ name: "Theo Marsh", frequencyDays: 90, lastConnected: "2026-09-29" }] });
  await openPreferences(page);
  await page.getByRole("button", { name: "Edit Theo Marsh" }).click();
  const form = page.getByRole("form", { name: "Save changes" });
  await form.getByLabel("Name").fill("x".repeat(100));
  await form.getByLabel("Last in touch").fill("1850-01-01");
  await form.getByRole("button", { name: "Save changes" }).click();
  await expect(form.getByRole("alert")).toHaveText("Pick a date between 1900 and today.");
  await expect(form).toBeVisible();
});

test("mark a person done from the list", async ({ page, request }) => {
  await seed(request, { people: SAMPLE.slice(0, 1) });
  await openPreferences(page);
  const row = page.getByTestId("person-row");
  await expect(row).toContainText("Due");
  await row.getByRole("button", { name: "Done" }).click();
  await expect(row).toContainText("Monthly · Reminder in 4 weeks, Mon, Dec 14");
  await expect(page.getByRole("tab", { name: "People" })).toHaveText("People");
});

test("saving an edit keeps a Done made while the form was open", async ({ page, request }) => {
  await seed(request, { people: SAMPLE.slice(0, 1) });
  await openPreferences(page);
  const row = page.getByTestId("person-row");
  await page.getByRole("button", { name: `Edit ${SAMPLE[0]!.name}` }).click();
  const form = page.getByRole("form", { name: "Save changes" });
  // With the form open, the person is marked done (as from the menu bar dropdown).
  await row.getByRole("button", { name: "Done" }).click();
  await expect(row).toContainText("Reminder in 4 weeks, Mon, Dec 14");
  await form.getByLabel("Name").fill("Maya C.");
  await form.getByRole("button", { name: "Save changes" }).click();
  await expect(form).toHaveCount(0);
  await expect(row).toContainText("Maya C.");
  await expect(row).toContainText("Monthly · Reminder in 4 weeks, Mon, Dec 14");
});

test("delete several people after confirming", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  await openPreferences(page);
  await page.getByLabel("Select Maya Chen").check();
  await page.getByLabel("Select Theo Marsh").check();
  const bar = page.getByRole("region", { name: "Selected people" });
  await expect(bar).toContainText("2 selected");
  await bar.getByRole("button", { name: "Delete selected…" }).click();
  await bar.getByRole("button", { name: "Keep" }).click();
  await expect(page.getByTestId("person-row")).toHaveCount(6);
  await bar.getByRole("button", { name: "Delete selected…" }).click();
  await bar.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByTestId("person-row")).toHaveCount(4);
  await expect(page.getByText("Maya Chen")).toHaveCount(0);
  await expect(bar).toHaveCount(0);
});

test("closing a form sends focus back to the button that opened it", async ({ page, request }) => {
  await seed(request, { people: [{ name: "Theo Marsh", frequencyDays: 90, lastConnected: "2026-09-29" }] });
  await openPreferences(page);
  const addButton = page.getByRole("button", { name: "Add person" });

  // Add with the keyboard: Enter in the form adds the person and closes it.
  await addButton.focus();
  await page.keyboard.press("Enter");
  const addForm = page.getByRole("form", { name: "Add person" });
  await expect(addForm.getByLabel("Name")).toBeFocused();
  await page.keyboard.type("Nora Quill");
  await page.keyboard.press("Enter");
  await expect(addForm).toHaveCount(0);
  await expect(page.getByTestId("person-row")).toHaveCount(2);
  await expect(addButton).toBeFocused();

  // Cancel goes back to Add person too.
  await addButton.click();
  await addForm.getByRole("button", { name: "Cancel" }).click();
  await expect(addForm).toHaveCount(0);
  await expect(addButton).toBeFocused();

  // Save changes goes back to that person's Edit button.
  const edit = page.getByRole("button", { name: "Edit Theo Marsh" });
  await edit.click();
  const editForm = page.getByRole("form", { name: "Save changes" });
  await editForm.getByRole("button", { name: "Save changes" }).click();
  await expect(editForm).toHaveCount(0);
  await expect(edit).toBeFocused();
});

test("deleting people sends focus to the person in their place, or to Add person", async ({ page, request }) => {
  await seed(request, {
    people: [
      { name: "Maya Chen", frequencyDays: 30, lastConnected: "2026-10-10" },
      { name: "Theo Marsh", frequencyDays: 90, lastConnected: "2026-09-29" },
      { name: "Lena Fischer", frequencyDays: 365, lastConnected: "2026-03-02" },
    ],
  });
  await openPreferences(page);
  const rows = page.getByTestId("person-row");
  await expect(rows).toHaveCount(3);

  // Delete Maya with the keyboard: focus goes to the checkbox of whoever now has her place.
  const maya = page.getByLabel("Select Maya Chen");
  await maya.focus();
  await page.keyboard.press("Space");
  const bar = page.getByRole("region", { name: "Selected people" });
  const deleteSelected = bar.getByRole("button", { name: "Delete selected…" });
  await deleteSelected.focus();
  await page.keyboard.press("Enter");
  await bar.getByRole("button", { name: "Delete", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(rows).toHaveCount(2);
  const first = rows.first().getByRole("checkbox");
  await expect(first).toBeFocused();

  // Delete everyone left: focus goes to Add person.
  await page.keyboard.press("Space");
  await rows.nth(1).getByRole("checkbox").check();
  await bar.getByRole("button", { name: "Delete selected…" }).click();
  await bar.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByText("Nobody yet")).toBeVisible();
  await expect(page.getByRole("button", { name: "Add person" })).toBeFocused();
});

test("Deselect all sends focus to the first selected person's checkbox", async ({ page, request }) => {
  await seed(request, {
    people: [
      { name: "Maya Chen", frequencyDays: 30, lastConnected: "2026-10-10" },
      { name: "Jonah Okafor", frequencyDays: 90, lastConnected: "2026-08-01" },
    ],
  });
  await openPreferences(page);
  const maya = page.getByLabel("Select Maya Chen");
  await maya.focus();
  await page.keyboard.press("Space");
  const bar = page.getByRole("region", { name: "Selected people" });
  await bar.getByRole("button", { name: "Deselect all" }).focus();
  await page.keyboard.press("Enter");
  await expect(bar).toHaveCount(0);
  await expect(maya).not.toBeChecked();
  await expect(maya).toBeFocused();
});

test("Done keeps focus on that person's Done button after the list re-sorts", async ({ page, request }) => {
  await seed(request, { people: SAMPLE });
  await openPreferences(page);
  const rows = page.getByTestId("person-row");
  await expect(rows.first()).toContainText("Jonah Okafor");
  await page.getByRole("button", { name: "Mark Jonah Okafor as done" }).focus();
  await page.keyboard.press("Enter");
  // Done today: Jonah's row moves down the list.
  await expect(rows.first()).not.toContainText("Jonah Okafor");
  await expect(page.getByRole("button", { name: "Mark Jonah Okafor as done" })).toBeFocused();
});
