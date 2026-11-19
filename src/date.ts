/** Calendar dates as the Rust side sends them ("YYYY-MM-DD"), shown in the user's language. */
import type { IsoDate } from "./api";

/** The date at local midnight, so formatting never shifts it by a time zone. */
export function parseIso(iso: IsoDate): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y ?? 1970, (m ?? 1) - 1, d ?? 1);
}

export function toIso(date: Date): IsoDate {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** Whole days from `from` to `to` (negative when `to` is earlier). */
export function daysBetween(from: IsoDate, to: IsoDate): number {
  return Math.round((parseIso(to).getTime() - parseIso(from).getTime()) / 86_400_000);
}

/** "Sat, Aug 22", with the year only when it isn't this year's. */
export function formatDay(iso: IsoDate, today: IsoDate): string {
  const date = parseIso(iso);
  const sameYear = date.getFullYear() === parseIso(today).getFullYear();
  return date.toLocaleDateString("en-US", {
    weekday: "short",
    month: "short",
    day: "numeric",
    ...(sameYear ? {} : { year: "numeric" }),
  });
}

/** "today", "tomorrow", "in 5 days", "in 3 weeks", "yesterday", "4 days ago". */
export function relativeDay(iso: IsoDate, today: IsoDate): string {
  const n = daysBetween(today, iso);
  if (n === 0) return "today";
  if (n === 1) return "tomorrow";
  if (n === -1) return "yesterday";
  if (n < 0) return `${-n} days ago`;
  if (n < 14) return `in ${n} days`;
  if (n < 60) return `in ${Math.round(n / 7)} weeks`;
  return `in ${Math.round(n / 30)} months`;
}

/** "Aug 20 – 27", "Aug 30 – Sep 2" or "Dec 28, 2026 – Jan 3, 2027". */
export function formatRange(start: IsoDate, end: IsoDate): string {
  const a = parseIso(start);
  const b = parseIso(end);
  const md = { month: "short", day: "numeric" } as const;
  const mdy = { ...md, year: "numeric" } as const;
  if (a.getFullYear() !== b.getFullYear()) {
    return `${a.toLocaleDateString("en-US", mdy)} – ${b.toLocaleDateString("en-US", mdy)}`;
  }
  if (start === end) return a.toLocaleDateString("en-US", md);
  if (a.getMonth() === b.getMonth()) return `${a.toLocaleDateString("en-US", md)} – ${b.getDate()}`;
  return `${a.toLocaleDateString("en-US", md)} – ${b.toLocaleDateString("en-US", md)}`;
}
