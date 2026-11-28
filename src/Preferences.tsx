/** The Preferences window: People, Blocked dates and Settings. */
import { type KeyboardEvent, useState } from "react";
import { BlockedPane } from "./BlockedPane";
import { WithGuide } from "./Guide";
import { PeoplePane } from "./PeoplePane";
import { SettingsPane } from "./SettingsPane";
import { useShownPerson } from "./useShownPerson";
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
  // A link from a contact card: People, at that person.
  const shown = useShownPerson(() => setTab("people"));
  const due = view?.people.filter((p) => p.due).length ?? 0;

  // A vertical tab list: Up and Down move to the previous and next tab (wrapping round), Home and
  // End to the first and last, and the tab moved to is shown straight away.
  // Counts from the focused tab: a contact card's link can select People while focus is elsewhere.
  const onTabKey = (e: KeyboardEvent) => {
    if (e.metaKey || e.altKey || e.ctrlKey || e.shiftKey) return;
    const focused = TABS.findIndex((t) => `tab-${t.id}` === (e.target as HTMLElement).id);
    const at = focused >= 0 ? focused : TABS.findIndex((t) => t.id === tab);
    const to =
      e.key === "ArrowDown" ? (at + 1) % TABS.length
      : e.key === "ArrowUp" ? (at - 1 + TABS.length) % TABS.length
      : e.key === "Home" ? 0
      : e.key === "End" ? TABS.length - 1
      : null;
    if (to === null) return;
    e.preventDefault();
    const next = TABS[to]!.id;
    setTab(next);
    document.getElementById(`tab-${next}`)?.focus();
  };

  const prefs = (
    <div className="prefs">
      <nav className="sidebar" aria-label="Preferences">
        <h1 className="wordmark">Friendship Reminder</h1>
        <div role="tablist" aria-orientation="vertical" onKeyDown={onTabKey}>
          {TABS.map((t) => (
            <button
              key={t.id}
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls={`pane-${t.id}`}
              // Only the selected tab is a Tab stop, so Tab goes from the tab list into its pane.
              tabIndex={tab === t.id ? 0 : -1}
              aria-label={t.id === "people" && due > 0 ? `${t.title}, ${due} due` : undefined}
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
                <PeoplePane view={view} shown={shown} />
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
  // A new user sees the first-run guide in place of the window's panes; then People's Add person.
  return view ? (
    <WithGuide view={view} landing="add-person">
      {prefs}
    </WithGuide>
  ) : (
    prefs
  );
}
