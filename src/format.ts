/** How often to be in touch, as offered in the app. */
export const FREQUENCIES = [
  { label: "Monthly", days: 30 },
  { label: "Quarterly", days: 90 },
  { label: "Twice a year", days: 180 },
  { label: "Yearly", days: 365 },
] as const;

export function frequencyLabel(days: number): string {
  return FREQUENCIES.find((f) => f.days === days)?.label ?? `Every ${days} days`;
}

/** How long ago `then` was (both in milliseconds since 1970), in words: "just now", "5 minutes ago". */
export function timeAgo(then: number, now: number): string {
  const minutes = Math.floor((now - then) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return minutes === 1 ? "1 minute ago" : `${minutes} minutes ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return hours === 1 ? "1 hour ago" : `${hours} hours ago`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "yesterday" : `${days} days ago`;
}
