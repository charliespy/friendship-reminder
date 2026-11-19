/** The Preferences window: People, Blocked dates and Settings. */
import { useState } from "react";
import { BlockedPane } from "./BlockedPane";
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

  return (
    <div className="prefs">
      <nav className="sidebar" aria-label="Preferences">
        <h1 className="wordmark">Friends</h1>
        <div role="tablist" aria-orientation="vertical">
          {TABS.map((t) => (
            <button
              key={t.id}
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls="pane"
              className="tab"
              onClick={() => setTab(t.id)}
            >
              {t.title}
              {t.id === "people" && due > 0 && <span className="tag">{due}</span>}
            </button>
          ))}
        </div>
      </nav>
      <main className="pane" id="pane" role="tabpanel" aria-labelledby={`tab-${tab}`}>
        {view?.unreadableFile && (
          <p className="banner" role="alert">
            Your saved list couldn't be read, so Friends started fresh. The old file is at {view.unreadableFile}.
          </p>
        )}
        {!view ? (
          <p className="empty" role={loadError ? "alert" : undefined}>
            {loadError ?? "Loading…"}
          </p>
        ) : tab === "people" ? (
          <PeoplePane view={view} />
        ) : tab === "blocked" ? (
          <BlockedPane view={view} />
        ) : (
          <SettingsPane view={view} />
        )}
      </main>
    </div>
  );
}
