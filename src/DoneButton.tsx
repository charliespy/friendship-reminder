/**
 * "Done" for one person, and the question it asks first: when were you in touch? The answer may not
 * be today (you met on the Saturday, and only opened Friends on Monday), and the next target counts
 * from it.
 */
import { useId, useLayoutEffect, useRef, useState, type Ref } from "react";
import { api, type IsoDate, type Person } from "./api";
import { formatDay } from "./date";
import { useAction } from "./useView";

export function DoneButton({
  person,
  today,
  buttonRef,
  onStart,
  onEnd,
}: {
  person: Person;
  today: IsoDate;
  buttonRef?: Ref<HTMLButtonElement>;
  /** Called just before the change is sent. */
  onStart?: (on: IsoDate) => void;
  /** Called once it has finished, with whether it worked. */
  onEnd?: (ok: boolean) => void;
}) {
  const { run, busy, error, setError } = useAction();
  const dialog = useRef<HTMLDialogElement>(null);
  // The question is drawn only while open, so closed ones add nothing to each row.
  const [shown, setShown] = useState(false);
  const [picking, setPicking] = useState(false);
  const [day, setDay] = useState(today);
  const titleId = useId();
  const dayId = useId();

  // The day suggested, when it has come and isn't today: offered as its own answer.
  const suggested = person.suggestedDay;
  const suggestedEarlier =
    suggested !== null && suggested < today ? suggested : null;

  const open = () => {
    setError(null);
    setPicking(false);
    setDay(today);
    setShown(true);
  };
  useLayoutEffect(() => {
    const d = dialog.current;
    if (!shown || !d || d.open) return;
    // jsdom has no showModal; the real webviews all do.
    if (typeof d.showModal === "function") d.showModal();
    else d.setAttribute("open", "");
  }, [shown]);
  const close = () => {
    const d = dialog.current;
    if (d && typeof d.close === "function") d.close();
    else d?.removeAttribute("open");
    setShown(false);
  };

  const done = async (on: IsoDate) => {
    onStart?.(on);
    const ok = await run(() =>
      api.markDone(person.id, on === today ? undefined : on),
    );
    if (ok) close();
    onEnd?.(ok);
  };

  return (
    <>
      <button
        ref={buttonRef}
        className="btn btn-done btn-sm"
        disabled={busy}
        onClick={open}
        aria-label={`Mark ${person.name} as done`}
      >
        Done
      </button>
      <dialog
        ref={dialog}
        className="done-dialog"
        aria-labelledby={titleId}
        // Escape closes it too.
        onClose={() => setShown(false)}
        onClick={(e) => {
          // A click on the dimmed backdrop (the dialog itself, outside its box) cancels.
          if (e.target === e.currentTarget) close();
        }}
      >
        {shown && (
          <div className="done-box">
            <h2 className="done-title" id={titleId}>
              When were you in touch with {person.name}?
            </h2>
            <div className="done-options">
              {suggestedEarlier && (
                <button
                  className="done-option"
                  disabled={busy}
                  onClick={() => done(suggestedEarlier)}
                >
                  <span className="choice-title">On the day suggested</span>
                  <span className="choice-text">
                    {formatDay(suggestedEarlier, today)}
                  </span>
                </button>
              )}
              <button
                className="done-option"
                disabled={busy}
                onClick={() => done(today)}
              >
                <span className="choice-title">Today</span>
                <span className="choice-text">
                  {formatDay(today, today)}
                  {suggested === today && " · the day suggested"}
                </span>
              </button>
              {picking ? (
                <form
                  className="done-option done-pick"
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (day) void done(day);
                  }}
                >
                  <label className="choice-title" htmlFor={dayId}>
                    Another day
                  </label>
                  <div className="done-pick-row">
                    <input
                      id={dayId}
                      type="date"
                      value={day}
                      // A stored day can be "tomorrow" after a time zone change.
                      min={person.lastConnected < today ? person.lastConnected : today}
                      max={today}
                      required
                      autoFocus
                      onChange={(e) => setDay(e.target.value)}
                    />
                    <button
                      type="submit"
                      className="btn btn-primary"
                      disabled={busy || !day}
                    >
                      Done
                    </button>
                  </div>
                </form>
              ) : (
                <button
                  className="done-option"
                  disabled={busy}
                  onClick={() => setPicking(true)}
                >
                  <span className="choice-title">Another day…</span>
                  <span className="choice-text">Pick it on a calendar.</span>
                </button>
              )}
            </div>
            {error && (
              <p className="error" role="alert">
                {error}
              </p>
            )}
            <button
              className="btn btn-quiet done-cancel"
              disabled={busy}
              onClick={close}
            >
              Cancel
            </button>
          </div>
        )}
      </dialog>
    </>
  );
}
