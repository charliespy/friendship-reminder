/**
 * "Done" for one person, and the question it asks first: when were you in touch? The answer may not
 * be today (you met on the Saturday, and only opened Friendship Reminder on Monday), and the next target counts
 * from it.
 */
import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type Ref,
} from "react";
import { api, type IsoDate, type Person } from "./api";
import { formatDay } from "./date";
import { useAction } from "./useView";

/** Closes the question's dialog (jsdom, in tests, has no `close`). */
function closeDialog(d: HTMLDialogElement | null) {
  if (d && typeof d.close === "function") d.close();
  else d?.removeAttribute("open");
}

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

  const suggestedEarlier = person.suggestedEarlier;

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
  // The menu bar list hides when it loses focus: an unanswered question goes with it, so it isn't
  // waiting there, with an old day, the next time the list opens.
  useEffect(() => {
    if (!shown) return;
    const onHidden = () => {
      if (document.visibilityState !== "hidden" || busy) return;
      // Closing fires the dialog's close event, which hides the question.
      closeDialog(dialog.current);
      setShown(false);
    };
    document.addEventListener("visibilitychange", onHidden);
    return () => document.removeEventListener("visibilitychange", onHidden);
  }, [shown, busy]);
  const close = () => {
    closeDialog(dialog.current);
    setShown(false);
  };

  const done = async (on: IsoDate) => {
    onStart?.(on);
    const ok = await run(() =>
      // Always the day shown: the app's own today may have moved on past midnight.
      api.markDone(person.id, on),
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
        // Not while the answer is being saved: an error would have nowhere to show.
        onCancel={(e) => {
          if (busy) e.preventDefault();
        }}
        // Escape is the question's alone: the menu bar list mustn't hide with it.
        onKeyDown={(e) => {
          if (e.key === "Escape") e.stopPropagation();
        }}
        onClick={(e) => {
          // A click on the dimmed backdrop (the dialog itself, outside its box) cancels.
          if (e.target === e.currentTarget && !busy) close();
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
                  {person.suggestedToday && " · the day suggested"}
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
                      min={person.earliestDone}
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
