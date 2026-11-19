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
