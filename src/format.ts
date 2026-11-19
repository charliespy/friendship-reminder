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
