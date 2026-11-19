import { describe, expect, it } from "vitest";
import { aimDay, daysBetween, formatDay, formatRange, parseIso, relativeDay, toIso } from "./date";

describe("dates", () => {
  it("round-trips ISO dates without shifting a day", () => {
    expect(toIso(parseIso("2026-08-22"))).toBe("2026-08-22");
    expect(toIso(parseIso("2026-03-08"))).toBe("2026-03-08"); // a DST change in the US
  });

  it("counts whole days", () => {
    expect(daysBetween("2026-08-19", "2026-08-22")).toBe(3);
    expect(daysBetween("2026-03-07", "2026-03-09")).toBe(2);
    expect(daysBetween("2026-08-22", "2026-08-19")).toBe(-3);
  });

  it("aims for today from the target day on", () => {
    expect(aimDay("2026-11-21", "2026-11-19")).toBe("Sat, Nov 21");
    expect(aimDay("2026-11-21", "2026-11-21")).toBe("today");
    expect(aimDay("2026-11-21", "2026-11-22")).toBe("today"); // the Sunday of a Saturday target
  });

  it("formats a day, with the year only when it differs", () => {
    expect(formatDay("2026-08-22", "2026-08-19")).toBe("Sat, Aug 22");
    expect(formatDay("2027-01-02", "2026-08-19")).toBe("Sat, Jan 2, 2027");
  });

  it("says how far off a day is", () => {
    expect(relativeDay("2026-08-19", "2026-08-19")).toBe("today");
    expect(relativeDay("2026-08-20", "2026-08-19")).toBe("tomorrow");
    expect(relativeDay("2026-08-18", "2026-08-19")).toBe("yesterday");
    expect(relativeDay("2026-08-15", "2026-08-19")).toBe("4 days ago");
    expect(relativeDay("2026-08-24", "2026-08-19")).toBe("in 5 days");
    expect(relativeDay("2026-09-09", "2026-08-19")).toBe("in 3 weeks");
    expect(relativeDay("2026-11-19", "2026-08-19")).toBe("in 3 months");
  });

  it("formats ranges compactly", () => {
    const today = "2026-11-18";
    expect(formatRange("2026-08-20", "2026-08-27", today)).toBe("Aug 20 – 27");
    expect(formatRange("2026-08-30", "2026-09-02", today)).toBe("Aug 30 – Sep 2");
    expect(formatRange("2026-08-20", "2026-08-20", today)).toBe("Aug 20");
    expect(formatRange("2026-12-28", "2027-01-03", today)).toBe("Dec 28, 2026 – Jan 3, 2027");
  });

  it("adds the year to ranges that aren't in this year", () => {
    const today = "2026-11-18";
    expect(formatRange("2025-12-01", "2025-12-05", today)).toBe("Dec 1 – 5, 2025");
    expect(formatRange("2027-12-01", "2027-12-05", today)).toBe("Dec 1 – 5, 2027");
    expect(formatRange("2027-11-30", "2027-11-30", today)).toBe("Nov 30, 2027");
    expect(formatRange("2027-08-30", "2027-09-02", today)).toBe("Aug 30 – Sep 2, 2027");
    expect(formatRange("2025-08-30", "2025-09-02", today)).toBe("Aug 30 – Sep 2, 2025");
    expect(formatRange("2024-12-28", "2025-01-03", today)).toBe("Dec 28, 2024 – Jan 3, 2025");
    expect(formatRange("2026-12-01", "2026-12-05", today)).toBe("Dec 1 – 5");
  });
});
