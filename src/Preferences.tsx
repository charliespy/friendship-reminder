/** The Preferences window: People, Blocked dates and Settings. */
import { useState } from "react";
import { BlockedPane } from "./BlockedPane";
import { WithGuide } from "./Guide";
import { PeoplePane } from "./PeoplePane";
import { SettingsPane } from "./SettingsPane";
import { useView } from "./useView";

type Tab = "people" | "blocked" | "settings";

const TABS: { id: Tab; title: string }[] = [
  { id: "people", title: "People" },
  { id: "blocked", title: "Blocked dates" },
  { id: "settings", title: "Settings" },
];

export function Preferences() {
  const { view, loadError } = useView();
  const [tab, setTab] = useState<Tab>("people");
  const due = view?.people.filter((p) => p.due).length ?? 0;

  const prefs = (
    <div className="prefs">
      <nav className="sidebar" aria-label="Preferences">
        <h1 className="wordmark">Friendship Reminder</h1>
        <div role="tablist" aria-orientation="vertical">
          {TABS.map((t) => (
            <button
              key={t.id}
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls={`pane-${t.id}`}
              className="tab"
              onClick={() => setTab(t.id)}
            >
              {t.title}
              {t.id === "people" && due > 0 && <span className="tag">{due}</span>}
            </button>
          ))}
        </div>
      </nav>
      <main className="pane">
        {view?.unreadableFile && (
          <p className="banner" role="alert">
            Your saved list couldn't be read, so Friendship Reminder started fresh. The old file is at {view.unreadableFile}.
          </p>
        )}
        {!view ? (
          <p className="empty" role={loadError ? "alert" : undefined}>
            {loadError ?? "Loading…"}
          </p>
        ) : (
          // Every pane stays mounted, so switching tabs keeps a half-filled form and a selection.
          TABS.map((t) => (
            <div
              key={t.id}
              id={`pane-${t.id}`}
              role="tabpanel"
              aria-labelledby={`tab-${t.id}`}
              hidden={tab !== t.id}
            >
              {t.id === "people" ? (
                <PeoplePane view={view} />
              ) : t.id === "blocked" ? (
                <BlockedPane view={view} />
              ) : (
                <SettingsPane view={view} />
              )}
            </div>
          ))
        )}
      </main>
    </div>
  );
  // A new user sees the first-run guide in place of the window's panes.
  return view ? <WithGuide view={view}>{prefs}</WithGuide> : prefs;
}
