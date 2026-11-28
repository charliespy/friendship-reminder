import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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

  it("still sends focus on when the window lost focus meanwhile (iOS asking about notifications)", async () => {
    const { rerender } = render(<Card shown />);
    const notNow = screen.getByRole("button", { name: "Not now" });
    notNow.focus();
    const hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(false);
    notNow.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await Promise.resolve();
    hasFocus.mockRestore();
    rerender(<Card shown={false} />);
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Today" }));
  });

  it("still sends focus on when a control inside went away and the rest stayed (Sign in, then Cancel)", () => {
    function Swap({ shown, step }: { shown: boolean; step: 1 | 2 }) {
      const focus = useFocusWhenGone(shown, () => document.getElementById("landing"));
      return (
        <>
          <h2 id="landing" tabIndex={-1}>
            Today
          </h2>
          {shown && <section {...focus}>{step === 1 ? <button>Sign in</button> : <button>Cancel</button>}</section>}
        </>
      );
    }
    const { rerender } = render(<Swap shown step={1} />);
    screen.getByRole("button", { name: "Sign in" }).focus();
    rerender(<Swap shown step={2} />);
    rerender(<Swap shown={false} step={2} />);
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Today" }));
  });

  it("sends focus to the new page when one page gives way to another (the guide's next page)", () => {
    function Pages({ page }: { page: "signIn" | "import" | null }) {
      const focus = useFocusWhenGone(page, () => document.getElementById(page ? "page-title" : "landing"));
      return (
        <>
          <h2 id="landing" tabIndex={-1}>
            Today
          </h2>
          {page && (
            <section {...focus}>
              <h1 id="page-title" tabIndex={-1}>
                {page === "signIn" ? "Welcome" : "Who?"}
              </h1>
              <button key={page}>{page === "signIn" ? "Not now" : "Skip"}</button>
            </section>
          )}
        </>
      );
    }
    const { rerender } = render(<Pages page="signIn" />);
    screen.getByRole("button", { name: "Not now" }).focus();
    rerender(<Pages page="import" />);
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Who?" }));
    // Still inside, so closing sends it on to the app.
    rerender(<Pages page={null} />);
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Today" }));
  });

  it("still sends focus on at closing when the new page's own field took focus by itself", () => {
    function Pages({ page }: { page: "signIn" | "import" | null }) {
      const focus = useFocusWhenGone(page, () => document.getElementById(page ? "page-title" : "landing"));
      return (
        <>
          <h2 id="landing" tabIndex={-1}>
            Today
          </h2>
          {page && (
            <section {...focus}>
              <h1 id="page-title" tabIndex={-1}>
                {page}
              </h1>
              {page === "signIn" ? <button>Not now</button> : <input aria-label="Search" autoFocus />}
            </section>
          )}
        </>
      );
    }
    const { rerender } = render(<Pages page="signIn" />);
    screen.getByRole("button", { name: "Not now" }).focus();
    rerender(<Pages page="import" />);
    const search = screen.getByRole("textbox", { name: "Search" });
    expect(document.activeElement).toBe(search);
    // The field goes away with the guide: focus goes to the app.
    rerender(<Pages page={null} />);
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Today" }));
  });

  it("sends focus on when the focused button turns off and keeps focus (Save, once saved)", () => {
    function Form({ state }: { state: "changed" | "saved" }) {
      const focus = useFocusWhenGone(state, () => document.querySelector<HTMLElement>("input:checked"));
      return (
        <fieldset {...focus}>
          <input type="radio" aria-label="Weekdays only" defaultChecked />
          <button disabled={state === "saved"}>Save</button>
        </fieldset>
      );
    }
    const { rerender } = render(<Form state="changed" />);
    screen.getByRole("button", { name: "Save" }).focus();
    rerender(<Form state="saved" />);
    expect(document.activeElement).toBe(screen.getByRole("radio", { name: "Weekdays only" }));
  });

  it("doesn't scroll to a button swapped in the same place (Sign out for Cancel)", () => {
    function Row({ row }: { row: "waiting" | "signedIn" }) {
      const focus = useFocusWhenGone(row, () => document.querySelector<HTMLElement>("section button"), { inPlace: true });
      return <section {...focus}>{row === "waiting" ? <button key="a">Cancel</button> : <button key="b">Sign out</button>}</section>;
    }
    const { rerender } = render(<Row row="waiting" />);
    screen.getByRole("button", { name: "Cancel" }).focus();
    const focus = vi.spyOn(HTMLElement.prototype, "focus");
    rerender(<Row row="signedIn" />);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Sign out" }));
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    focus.mockRestore();
  });
});
