import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useFocusWhenGone } from "./useFocusWhenGone";

function Card({ shown }: { shown: boolean }) {
  const focus = useFocusWhenGone(shown, () => document.getElementById("landing"));
  return (
    <>
      <h2 id="landing" tabIndex={-1}>
        Today
      </h2>
      <button>Elsewhere</button>
      {shown && (
        <section {...focus}>
          <button>Not now</button>
        </section>
      )}
    </>
  );
}

afterEach(cleanup);

describe("useFocusWhenGone", () => {
  it("sends focus to the target when it goes away with the focused button", () => {
    const { rerender } = render(<Card shown />);
    screen.getByRole("button", { name: "Not now" }).focus();
    rerender(<Card shown={false} />);
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Today" }));
  });

  it("moves nothing when focus wasn't inside (a tap on iPhone focuses nothing)", () => {
    const { rerender } = render(<Card shown />);
    rerender(<Card shown={false} />);
    expect(document.activeElement).toBe(document.body);
  });

  it("never takes focus from somewhere the user moved it", () => {
    const { rerender } = render(<Card shown />);
    screen.getByRole("button", { name: "Not now" }).focus();
    const elsewhere = screen.getByRole("button", { name: "Elsewhere" });
    elsewhere.focus();
    rerender(<Card shown={false} />);
    expect(document.activeElement).toBe(elsewhere);
  });

  it("moves nothing when focus left it for the page's background", async () => {
    const { rerender } = render(<Card shown />);
    const notNow = screen.getByRole("button", { name: "Not now" });
    notNow.focus();
    notNow.blur();
    await Promise.resolve();
    rerender(<Card shown={false} />);
    expect(document.activeElement).toBe(document.body);
  });
});
