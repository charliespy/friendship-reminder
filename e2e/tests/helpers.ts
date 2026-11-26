import { expect, type APIRequestContext, type Page } from "@playwright/test";
import { SERVER_PORT } from "../ports";

const server = `http://127.0.0.1:${SERVER_PORT}`;
/** The devserver only takes JSON posts (so a web page can't drive it). */
const JSON_HEADERS = { "Content-Type": "application/json" };

/** The fixed "today" the server starts with: Wednesday, Nov 18, 2026. */
export const TODAY = "2026-11-18";

export interface SeedPerson {
  name: string;
  frequencyDays: 30 | 90 | 180 | 365;
  lastConnected: string;
}

async function call(request: APIRequestContext, cmd: string, args: Record<string, unknown>) {
  const res = await request.post(`${server}/invoke`, { data: { cmd: "call", args: { cmd, args } }, headers: JSON_HEADERS });
  expect(res.ok(), await res.text()).toBeTruthy();
  return res.json();
}

/** Starts from empty data on `today`, then adds the people and blocked ranges through the real commands. */
export async function seed(
  request: APIRequestContext,
  opts: { today?: string; people?: SeedPerson[]; blocked?: { start: string; end: string; label: string }[] } = {},
) {
  expect((await request.post(`${server}/__test/reset`, { data: {}, headers: JSON_HEADERS })).ok()).toBeTruthy();
  await setToday(request, opts.today ?? TODAY);
  for (const p of opts.people ?? []) await call(request, "add_person", { ...p });
  for (const b of opts.blocked ?? []) await call(request, "add_blocked", { ...b });
}

/** Answers Done's question, "When were you in touch?", with the answer whose name starts `answer`. */
export async function answerDone(page: Page, answer: "Today" | "On the day suggested" | "Another day") {
  await page.getByRole("dialog").getByRole("button", { name: new RegExp(`^${answer}`) }).click();
}

export async function setToday(request: APIRequestContext, day: string) {
  expect((await request.post(`${server}/__test/today`, { data: JSON.stringify(day), headers: JSON_HEADERS })).ok()).toBeTruthy();
}

/**
 * Three people whose weekend (Nov 14-15) went by, so they're due and missed, and three coming up.
 * The same list the README screenshots use.
 */
export const SAMPLE: SeedPerson[] = [
  { name: "Maya Chen", frequencyDays: 30, lastConnected: "2026-10-10" },
  { name: "Jonah Okafor", frequencyDays: 90, lastConnected: "2026-08-01" },
  { name: "Priya Raman", frequencyDays: 30, lastConnected: "2026-10-01" },
  { name: "Sam Whitfield", frequencyDays: 180, lastConnected: "2026-06-20" },
  { name: "Lena Fischer", frequencyDays: 365, lastConnected: "2026-03-02" },
  { name: "Theo Marsh", frequencyDays: 90, lastConnected: "2026-09-29" },
];

export async function openPreferences(page: Page, tab?: "People" | "Blocked dates" | "Settings") {
  await page.goto("/preferences.html");
  if (tab) await page.getByRole("tab", { name: tab }).click();
}

/** Starts as a brand-new install: empty data, and the first-run guide not done. */
export async function seedNewUser(request: APIRequestContext) {
  const fresh = { version: 3, guide: { signIn: false, import: false } };
  expect((await request.post(`${server}/__test/reset`, { data: fresh, headers: JSON_HEADERS })).ok()).toBeTruthy();
  await setToday(request, TODAY);
}

/**
 * Contacts' access in the made-up address book (see friends-devserver): not asked yet (and what the
 * user will answer), all shared, only some shared (Ben, Elif and Iris; "share more" adds Kofi and
 * Lena), or refused. Each reset starts at "notAsked".
 */
export async function setContactsAccess(
  request: APIRequestContext,
  answer: "notAsked" | "notAskedLimited" | "allowed" | "limited" | "denied",
) {
  const res = await request.post(`${server}/__test/contacts`, { data: JSON.stringify(answer), headers: JSON_HEADERS });
  expect(res.ok()).toBeTruthy();
}

/**
 * Acts as if the app's link on a made-up contact card opened the app; answers the person's id, or
 * null when the card has no link to someone in the list.
 */
export async function openCard(request: APIRequestContext, card: string): Promise<number | null> {
  const res = await request.post(`${server}/__test/open-card`, { data: JSON.stringify(card), headers: JSON_HEADERS });
  expect(res.ok()).toBeTruthy();
  return res.json();
}
