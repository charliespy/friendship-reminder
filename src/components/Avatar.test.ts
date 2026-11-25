import { describe, expect, it } from "vitest";
import { initialOf } from "./Avatar";

describe("Avatar helpers", () => {
  it("takes the first letter, upper-cased, or ? for no name", () => {
    expect(initialOf("  ada")).toBe("A");
    expect(initialOf("élodie")).toBe("É");
    expect(initialOf("")).toBe("?");
  });
});
