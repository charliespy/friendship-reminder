/** The menu bar dropdown: who to reach out to now, who's coming up, and the way to Preferences. */
import { useEffect } from "react";
import { api, type Person, type View } from "./api";
import { Avatar } from "./components/Avatar";
import { aimDay, formatDay, relativeDay } from "./date";
import { useAction, useView } from "./useView";

/** How many upcoming people the dropdown lists. */
const UPCOMING = 5;

function dueLine(p: Person, today: string): string {
  const day = aimDay(p.nextTarget, today);
  return p.overdue ? `Missed last time · aim for ${day}` : `Aim for ${day}`;
}

function DueRow({ person, today }: { person: Person; today: string }) {
  const { run, busy, error } = useAction();
  return (
    <li className="person-row" data-testid="due-person" data-overdue={person.overdue}>
      <Avatar name={person.name} />
      <div className="person-text">
        <div className="person-name">{person.name}</div>
        <div className="person-meta">{dueLine(person, today)}</div>
        {error && <p className="error" role="alert">{error}</p>}
      </div>
      <button
        className="btn btn-done btn-sm"
        disabled={busy}
        onClick={() => run(() => api.markDone(person.id))}
        aria-label={`Mark ${person.name} as done`}
      >
        Done
      </button>
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

export function DropdownContent({ view }: { view: View }) {
  const due = view.people.filter((p) => p.due);
  const upcoming = view.people.filter((p) => !p.due).slice(0, UPCOMING);
  return (
    <>
      <div className="dropdown-body">
        {view.unreadableFile && (
          <p className="banner" role="alert">
            Your saved list couldn't be read, so Friends started fresh. The old file is at {view.unreadableFile}.
          </p>
        )}
        {view.people.length === 0 ? (
          <div className="empty">
            <strong>Nobody here yet</strong>
            Add the people you want to keep up with in Preferences.
          </div>
        ) : (
          <>
            <section className="dropdown-section" aria-labelledby="due-label">
              <h2 className="section-label" id="due-label">
                Reach out now
              </h2>
              {due.length === 0 ? (
                <p className="caught-up">You're all caught up.</p>
              ) : (
                <ul className="list due-list">
                  {due.map((p) => (
                    <DueRow key={p.id} person={p} today={view.today} />
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
