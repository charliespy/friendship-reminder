/**
 * The iPhone app: the Mac's dropdown and Preferences panes as four tabs, with a tab bar at the bottom.
 */
import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { BlockedPane } from "./BlockedPane";
import { formatDay } from "./date";
import { WithGuide } from "./Guide";
import { ReachOut } from "./Dropdown";
import { PeoplePane } from "./PeoplePane";
import { SettingsPane } from "./SettingsPane";
import { NotifyCard } from "./NotifyCard";
import { useNotificationAccess } from "./useNotificationAccess";
import { useShownPerson } from "./useShownPerson";
import { useView } from "./useView";

type Tab = "today" | "people" | "blocked" | "settings";

/** Tab bar symbols, drawn like SF Symbols: 24 pt, 1.75 pt strokes in the label colour. */
const ICONS: Record<Tab, ReactNode> = {
  today: (
    <>
      <rect x="3.5" y="5" width="17" height="15" rx="3" />
      <path d="M3.5 10h17M8 3v4M16 3v4" />
      <path d="m9 15 2 2 4-4" />
    </>
  ),
  people: (
    <>
      <circle cx="9" cy="8.5" r="3.25" />
      <path d="M3 19.5c.6-3.3 3-5 6-5s5.4 1.7 6 5" />
      <circle cx="16.5" cy="9" r="2.5" />
      <path d="M16.5 14.5c2.3 0 4 1.3 4.5 4" />
    </>
  ),
  blocked: (
    <>
      <rect x="3.5" y="5" width="17" height="15" rx="3" />
      <path d="M3.5 10h17M8 3v4M16 3v4" />
      <path d="M9 15h6" />
    </>
  ),
  settings: (
    <>
      {/* A gear: eight teeth (a dashed ring) around a ring, around the hole. */}
      <circle cx="12" cy="12" r="8.6" strokeWidth="2.4" strokeDasharray="2.6 4.15" strokeLinecap="butt" />
      <circle cx="12" cy="12" r="6.6" />
      <circle cx="12" cy="12" r="2.5" />
    </>
  ),
};

const TABS: { id: Tab; title: string }[] = [
  { id: "today", title: "Today" },
  { id: "people", title: "People" },
  { id: "blocked", title: "Blocked" },
  { id: "settings", title: "Settings" },
];

/** What scrolls: the page, or the phone-sized frame in browser dev mode. */
function scroller(): Element | null {
  return document.documentElement.classList.contains("in-browser-phone")
    ? document.getElementById("root")
    : document.scrollingElement;
}

export function Phone() {
  const { view, loadError } = useView();
  const [tab, setTab] = useState<Tab>("today");
  const due = view?.people.filter((p) => p.due).length ?? 0;
  // A link from a contact card: the People tab, at that person.
  const shown = useShownPerson(() => setTab("people"));
  // iOS asks about notifications only when the user turns reminders on (Today's card, or Settings).
  const notify = useNotificationAccess();
  const todayTitle = useRef<HTMLHeadingElement>(null);

  // Each tab keeps its own place: the page scrolls as a whole, so leaving a tab saves how far down
  // it was, and coming back puts it there again (the top, the first time).
  const scrolled = useRef<Partial<Record<Tab, number>>>({});
  useLayoutEffect(() => {
    scroller()?.scrollTo?.(0, scrolled.current[tab] ?? 0);
  }, [tab]);
  const open = (next: Tab) => {
    const page = scroller();
    if (next === tab) {
      // Tapping the open tab again goes back to its top, as in iOS apps.
      page?.scrollTo?.({ top: 0, behavior: "smooth" });
      return;
    }
    scrolled.current[tab] = page?.scrollTop ?? 0;
    setTab(next);
  };

  const app = (
    <div className="phone">
      <main className="phone-pane">
        {!view ? (
          <p className="empty" role={loadError ? "alert" : undefined}>
            {loadError ?? "Loading…"}
          </p>
        ) : (
          // Every tab stays mounted, so switching keeps a half-filled form and a selection.
          TABS.map((t) => (
            <div
              key={t.id}
              id={`pane-${t.id}`}
              role="tabpanel"
              aria-labelledby={`tab-${t.id}`}
              hidden={tab !== t.id}
            >
              {t.id === "today" ? (
                <section aria-labelledby="today-title">
                  <header className="pane-header">
                    <div>
                      {/* Takes focus when the guide or the card below goes away with the focused button. */}
                      <h2 id="today-title" ref={todayTitle} tabIndex={-1}>
                        Today
                      </h2>
                      <p>{formatDay(view.today, view.today)}</p>
                    </div>
                  </header>
                  <NotifyCard view={view} notify={notify} afterClose={() => todayTitle.current} />
                  <ReachOut view={view} addHint="Add the people you want to keep up with in People." />
                </section>
              ) : t.id === "people" ? (
                <PeoplePane view={view} shown={shown} />
              ) : t.id === "blocked" ? (
                <BlockedPane view={view} />
              ) : (
                <SettingsPane view={view} phone access={notify} />
              )}
            </div>
          ))
        )}
      </main>
      <nav className="tabbar" aria-label="Friendship Reminder">
        <div role="tablist">
          {TABS.map((t) => (
            <button
              key={t.id}
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls={`pane-${t.id}`}
              className="tabbar-item"
              aria-label={t.id === "today" && due > 0 ? `${t.title}, ${due} due` : undefined}
              onClick={() => open(t.id)}
            >
              <span className="tabbar-icon" aria-hidden="true">
                <svg viewBox="0 0 24 24">{ICONS[t.id]}</svg>
                {t.id === "today" && due > 0 && <span className="tabbar-badge">{due}</span>}
              </span>
              {t.title}
            </button>
          ))}
        </div>
      </nav>
    </div>
  );
  // A new user sees the first-run guide in place of the tabs; then Today's heading.
  return view ? (
    <WithGuide view={view} phone landing="today-title">
      {app}
    </WithGuide>
  ) : (
    app
  );
}
