import { describe, expect, it } from "vitest";
import { frequencyLabel, timeAgo } from "./format";

describe("frequencyLabel", () => {
  it("names the frequencies the app offers", () => {
    expect(frequencyLabel(30)).toBe("Monthly");
    expect(frequencyLabel(90)).toBe("Quarterly");
    // "Semiannual" in the Swift app, said more plainly.
    expect(frequencyLabel(180)).toBe("Twice a year");
    expect(frequencyLabel(365)).toBe("Yearly");
  });

  it("falls back to a day count", () => {
    expect(frequencyLabel(45)).toBe("Every 45 days");
  });
});

describe("timeAgo", () => {
  it("says how long ago, in words", () => {
    const now = 1_000_000_000_000;
    const minute = 60_000;
    expect(timeAgo(now - 30_000, now)).toBe("just now");
    expect(timeAgo(now - minute, now)).toBe("1 minute ago");
    expect(timeAgo(now - 59 * minute, now)).toBe("59 minutes ago");
    expect(timeAgo(now - 60 * minute, now)).toBe("1 hour ago");
    expect(timeAgo(now - 5 * 60 * minute, now)).toBe("5 hours ago");
    expect(timeAgo(now - 24 * 60 * minute, now)).toBe("yesterday");
    expect(timeAgo(now - 72 * 60 * minute, now)).toBe("3 days ago");
    // A clock that went back reads as just now, not a negative time.
    expect(timeAgo(now + minute, now)).toBe("just now");
  });
});
