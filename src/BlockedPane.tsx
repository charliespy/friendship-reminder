/** Preferences › Blocked dates: trips and busy stretches where no reminder should land. */
import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, type View } from "./api";
import { formatRange } from "./date";
import { useAction } from "./useView";

export function BlockedPane({ view }: { view: View }) {
  const [start, setStart] = useState(view.today);
  const [end, setEnd] = useState(view.today);
  const [label, setLabel] = useState("");
  const add = useAction();
  // Preferences stays open in the background for days: until the dates are touched, keep them on today.
  const touched = useRef(false);
  useEffect(() => {
    if (touched.current) return;
    setStart(view.today);
    setEnd(view.today);
  }, [view.today]);
  const remove = useAction();

  // Remove takes away its row, and the focused button with it. Focus then goes to the Remove button
  // of the range that took its place (or the one before it, if it was the last), or to the form's
  // first field when no ranges are left, so keyboard users don't land back at the top of the window.
  const firstField = useRef<HTMLInputElement>(null);
  const removeButtons = useRef(new Map<number, HTMLButtonElement>());
  const returnFocus = useRef<{ removed: number; next: number | null; finished: boolean } | null>(null);
  useEffect(() => {
    const target = returnFocus.current;
    // The new view can come before or after the command returns (the macOS app sends it first):
    // wait for both, so the Remove buttons are enabled again, and for a view without the range.
    if (target === null || !target.finished || view.blocked.some((b) => b.id === target.removed)) return;
    returnFocus.current = null;
    // Only if focus was lost with the row; never take it from somewhere the user moved it.
    if (document.activeElement && document.activeElement !== document.body) return;
    const next = target.next === null ? undefined : removeButtons.current.get(target.next);
    (next ?? firstField.current)?.focus();
  });

  const removeRange = async (id: number) => {
    // Who takes its place: the next range, else the one before.
    const index = view.blocked.findIndex((b) => b.id === id);
    const next = view.blocked[index + 1] ?? view.blocked[index - 1];
    const target = { removed: id, next: next?.id ?? null, finished: false };
    returnFocus.current = target;
    const ok = await remove.run(() => api.deleteBlocked(id));
    if (returnFocus.current !== target) return;
    // Finished: the busy state clearing re-renders the pane, which runs the effect above.
    if (ok) target.finished = true;
    else returnFocus.current = null;
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (await add.run(() => api.addBlocked({ start, end, label: label.trim() }))) {
      // Back to a fresh form: today's dates, which follow the calendar again until touched.
      setLabel("");
      setStart(view.today);
      setEnd(view.today);
      touched.current = false;
    }
  };

  return (
    <section aria-labelledby="blocked-title">
      <header className="pane-header">
        <div>
          <h2 id="blocked-title">Blocked dates</h2>
          <p>Trips or busy stretches. Anyone due then moves to the first free day after.</p>
        </div>
      </header>

      <form className="card form-card" onSubmit={submit} aria-label="Add blocked dates" noValidate>
        <div className="form-row">
          <label className="field grow">
            <span>What</span>
            <input
              ref={firstField}
              className="input"
              value={label}
              maxLength={100}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="Trip, busy week…"
            />
          </label>
          <label className="field">
            <span>From</span>
            <input
              className="input"
              type="date"
              value={start}
              min="1900-01-01"
              max="2200-12-31"
              onChange={(e) => {
                touched.current = true;
                setStart(e.target.value);
                if (e.target.value > end) setEnd(e.target.value);
              }}
            />
          </label>
          <label className="field">
            <span>To</span>
            <input
              className="input"
              type="date"
              value={end}
              min={start}
              max="2200-12-31"
              onChange={(e) => ((touched.current = true), setEnd(e.target.value))}
            />
          </label>
        </div>
        {add.error && (
          <p className="error" role="alert">
            {add.error}
          </p>
        )}
        <div className="form-actions">
          <button type="submit" className="btn btn-primary" disabled={add.busy || !start || !end}>
            Block dates
          </button>
        </div>
      </form>

      {remove.error && (
        <p className="error" role="alert">
          {remove.error}
        </p>
      )}
      {view.blocked.length === 0 ? (
        <div className="card empty">
          <strong>No blocked dates</strong>
          Every allowed day is fair game.
        </div>
      ) : (
        <ul className="list card">
          {view.blocked.map((b) => {
            const past = b.end < view.today;
            const range = formatRange(b.start, b.end, view.today);
            return (
              <li className="list-row" key={b.id} data-testid="blocked-row">
                <div className="person-text">
                  <div className="person-name">{b.label || "Blocked"}</div>
                  <div className="person-meta">
                    {range}
                    {past && " · over"}
                  </div>
                </div>
                <button
                  ref={(el) => {
                    if (el) removeButtons.current.set(b.id, el);
                    else removeButtons.current.delete(b.id);
                  }}
                  className="btn btn-quiet btn-sm"
                  disabled={remove.busy}
                  onClick={() => removeRange(b.id)}
                  aria-label={`Remove ${b.label || "blocked dates"} ${range}`}
                >
                  Remove
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
