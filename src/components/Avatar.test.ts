import { describe, expect, it } from "vitest";
import { initialOf, toneFor } from "./Avatar";

describe("Avatar helpers", () => {
  it("takes the first letter, upper-cased, or ? for no name", () => {
    expect(initialOf("  ada")).toBe("A");
    expect(initialOf("élodie")).toBe("É");
    expect(initialOf("")).toBe("?");
  });

  it("gives a name the same tone every time", () => {
    expect(toneFor("Ada")).toBe(toneFor("Ada"));
    expect(toneFor("Ada")).toBeGreaterThanOrEqual(0);
    expect(toneFor("Ada")).toBeLessThan(4);
  });
});
