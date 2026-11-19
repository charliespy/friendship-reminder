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
            return (
              <li className="list-row" key={b.id} data-testid="blocked-row">
                <div className="person-text">
                  <div className="person-name">{b.label || "Blocked"}</div>
                  <div className="person-meta">
                    {formatRange(b.start, b.end)}
                    {past && " · over"}
                  </div>
                </div>
                <button
                  className="btn btn-quiet btn-sm"
                  disabled={remove.busy}
                  onClick={() => remove.run(() => api.deleteBlocked(b.id))}
                  aria-label={`Remove ${b.label || "blocked dates"} ${formatRange(b.start, b.end)}`}
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
