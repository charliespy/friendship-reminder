import { describe, expect, it } from "vitest";
import { matches } from "./ContactPicker";

describe("matches (searching Contacts)", () => {
  it("ignores accent marks", () => {
    expect(matches("Chloé Martin", "chloe")).toBe(true);
    expect(matches("Chloe Martin", "Chloé")).toBe(true);
  });

  it("finds letters that are not a plain letter plus an accent", () => {
    expect(matches("Elif Yılmaz", "yilmaz")).toBe(true);
    expect(matches("Elif Yılmaz", "Yilmaz")).toBe(true);
    expect(matches("Łukasz Nowak", "lukasz")).toBe(true);
    expect(matches("Søren Berg", "soren")).toBe(true);
    expect(matches("Đorđe Ilić", "dorde")).toBe(true);
    expect(matches("Jörg Strauß", "strauss")).toBe(true);
    expect(matches("Æsa Brænd", "aesa braend")).toBe(true);
    expect(matches("Chloë Cœur", "coeur")).toBe(true);
  });

  it("still finds names typed with those letters", () => {
    expect(matches("Elif Yılmaz", "yılmaz")).toBe(true);
    expect(matches("Elif Yilmaz", "yılmaz")).toBe(true);
    expect(matches("Jörg Strauss", "Strauß")).toBe(true);
  });

  it("matches words in any order, and needs every word", () => {
    expect(matches("Elif Yılmaz", "yilmaz elif")).toBe(true);
    expect(matches("Elif Yılmaz", "yilmaz maya")).toBe(false);
    expect(matches("Søren Berg", "sarah")).toBe(false);
  });
});
