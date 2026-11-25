/** The menu bar dropdown: who to reach out to now, who's coming up, and the way to Preferences. */
import { useEffect, useRef } from "react";
import { api, type Person, type View } from "./api";
import { Avatar } from "./components/Avatar";
import { DoneButton } from "./DoneButton";
import { aimDay, formatDay, relativeDay } from "./date";
import { useView } from "./useView";

/** How many upcoming people the dropdown lists. */
const UPCOMING = 5;

function dueLine(p: Person, today: string): string {
  const day = aimDay(p.nextTarget, today);
  return p.overdue ? `Missed last time · aim for ${day}` : `Aim for ${day}`;
}

function DueRow({
  person,
  today,
  doneRef,
  onDoneStart,
  onDoneFailed,
}: {
  person: Person;
  today: string;
  doneRef: (el: HTMLButtonElement | null) => void;
  onDoneStart: () => void;
  onDoneFailed: () => void;
}) {
  return (
    <li className="person-row" data-testid="due-person" data-overdue={person.overdue}>
      <Avatar name={person.name} />
      <div className="person-text">
        <div className="person-name">{person.name}</div>
        <div className="person-meta">{dueLine(person, today)}</div>
      </div>
      <DoneButton
        person={person}
        today={today}
        buttonRef={doneRef}
        onStart={onDoneStart}
        onEnd={(ok) => {
          if (!ok) onDoneFailed();
        }}
      />
    </li>
  );
}

function UpcomingRow({ person, today }: { person: Person; today: string }) {
  return (
    <li className="person-row" data-testid="upcoming-person">
      <Avatar name={person.name} />
      <div className="person-text">
        <div className="person-name">{person.name}</div>
        <div className="person-meta">
          Reminder {relativeDay(person.notificationDate, today)} · {formatDay(person.notificationDate, today)}
        </div>
      </div>
    </li>
  );
}

/**
 * Who to reach out to now and who's coming up: the dropdown's list, and the iPhone's Today tab.
 * `addHint` says where to add people.
 */
export function ReachOut({ view, addHint }: { view: View; addHint: string }) {
  const due = view.people.filter((p) => p.due);
  const upcoming = view.people.filter((p) => !p.due).slice(0, UPCOMING);

  // A Done moves its row out of "Reach out now", taking the focused button with it. Focus then goes
  // to the Done button of the row that took its place (or the one before it), or to the
  // caught-up message when nobody is left, so keyboard users don't land back at the top.
  const doneButtons = useRef(new Map<number, HTMLButtonElement>());
  const caughtUp = useRef<HTMLParagraphElement>(null);
  const doneAt = useRef<{ id: number; index: number } | null>(null);
  useEffect(() => {
    const pending = doneAt.current;
    if (!pending || due.some((p) => p.id === pending.id)) return;
    doneAt.current = null;
    // Only if focus was lost with the row; never take it from somewhere the user moved it.
    if (document.activeElement && document.activeElement !== document.body) return;
    const next = due[Math.min(pending.index, due.length - 1)];
    (next ? doneButtons.current.get(next.id) : caughtUp.current)?.focus();
  });

  return (
    <div className="dropdown-body">
      {view.unreadableFile && (
        <p className="banner" role="alert">
          Your saved list couldn't be read, so Friends started fresh. The old file is at {view.unreadableFile}.
        </p>
      )}
      {view.people.length === 0 ? (
        <div className="empty">
          <strong>Nobody here yet</strong>
          {addHint}
        </div>
      ) : (
        <>
          <section className="dropdown-section" aria-labelledby="due-label">
            <h2 className="section-label" id="due-label">
              Reach out now
            </h2>
            {due.length === 0 ? (
              <p className="caught-up" ref={caughtUp} tabIndex={-1}>
                You're all caught up.
              </p>
            ) : (
              <ul className="list due-list">
                {due.map((p, index) => (
                  <DueRow
                    key={p.id}
                    person={p}
                    today={view.today}
                    doneRef={(el) => {
                      if (el) doneButtons.current.set(p.id, el);
                      else doneButtons.current.delete(p.id);
                    }}
                    onDoneStart={() => (doneAt.current = { id: p.id, index })}
                    onDoneFailed={() => {
                      if (doneAt.current?.id === p.id) doneAt.current = null;
                    }}
                  />
                ))}
              </ul>
            )}
          </section>
          {upcoming.length > 0 && (
            <section className="dropdown-section" aria-labelledby="upcoming-label">
              <h2 className="section-label" id="upcoming-label">
                Coming up
              </h2>
              <ul className="list">
                {upcoming.map((p) => (
                  <UpcomingRow key={p.id} person={p} today={view.today} />
                ))}
              </ul>
            </section>
          )}
        </>
      )}
    </div>
  );
}

export function DropdownContent({ view }: { view: View }) {
  return (
    <>
      <ReachOut view={view} addHint="Add the people you want to keep up with in Preferences." />
      <footer className="dropdown-footer">
        <button className="btn btn-quiet btn-sm" onClick={() => api.openPreferences()}>
          Preferences…
        </button>
        <button className="btn btn-quiet btn-sm" onClick={() => api.quit()}>
          Quit Friends
        </button>
      </footer>
    </>
  );
}

export function Dropdown() {
  const { view, loadError } = useView();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Escape in Done's question closes just the question (it stops the key there).
      if (e.key === "Escape") api.hideDropdown();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <main className="dropdown">
      <header className="dropdown-header">
        <h1 className="wordmark">Friends</h1>
        {view && <span className="muted">{formatDay(view.today, view.today)}</span>}
      </header>
      {view ? (
        <DropdownContent view={view} />
      ) : (
        <p className="empty" role={loadError ? "alert" : undefined}>
          {loadError ?? "Loading…"}
        </p>
      )}
    </main>
  );
}
