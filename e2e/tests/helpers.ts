import { expect, type APIRequestContext, type Page } from "@playwright/test";

const server = `http://127.0.0.1:${process.env.FRIENDS_E2E_SERVER_PORT ?? 1430}`;

/** The fixed "today" the server starts with: Wednesday, Nov 18, 2026. */
export const TODAY = "2026-11-18";

export interface SeedPerson {
  name: string;
  frequencyDays: 30 | 90 | 180 | 365;
  lastConnected: string;
}

async function call(request: APIRequestContext, cmd: string, args: Record<string, unknown>) {
  const res = await request.post(`${server}/invoke`, { data: { cmd: "call", args: { cmd, args } } });
  expect(res.ok(), await res.text()).toBeTruthy();
  return res.json();
}

/** Starts from empty data on `today`, then adds the people and blocked ranges through the real commands. */
export async function seed(
  request: APIRequestContext,
  opts: { today?: string; people?: SeedPerson[]; blocked?: { start: string; end: string; label: string }[] } = {},
) {
  expect((await request.post(`${server}/__test/reset`, { data: {} })).ok()).toBeTruthy();
  expect((await request.post(`${server}/__test/today`, { data: JSON.stringify(opts.today ?? TODAY) })).ok()).toBeTruthy();
  for (const p of opts.people ?? []) await call(request, "add_person", { ...p });
  for (const b of opts.blocked ?? []) await call(request, "add_blocked", { ...b });
}

export async function setToday(request: APIRequestContext, day: string) {
  expect((await request.post(`${server}/__test/today`, { data: JSON.stringify(day) })).ok()).toBeTruthy();
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
