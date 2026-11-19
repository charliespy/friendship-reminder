import { describe, expect, it } from "vitest";
import { frequencyLabel } from "./format";

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
