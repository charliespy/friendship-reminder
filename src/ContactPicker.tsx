/**
 * Picking people from Contacts: search the cards, tick one or several, pick how often for each, and
 * add them.
 * Used from Add person and from the first-run guide. The app reads only names (see SECURITY.md).
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { api, errorMessage, type ContactChoice } from "./api";
import { Avatar } from "./components/Avatar";
import { FREQUENCIES } from "./format";
import { useAction } from "./useView";

type Loaded =
  | { status: "loading" }
  | { status: "allowed"; contacts: ContactChoice[] }
  | { status: "denied" }
  | { status: "failed"; message: string };

/** How a search matches a name: any order of words, ignoring case and accents. */
function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

function matches(name: string, query: string): boolean {
  const n = fold(name);
  return fold(query).split(/\s+/).filter(Boolean).every((w) => n.includes(w));
}

/** How often to be in touch with someone just ticked: monthly, as in Add person. */
const DEFAULT_FREQUENCY = 30;

/** The iPhone app (its entry marks the page). */
function onPhone(): boolean {
  return document.documentElement.classList.contains("phone-app");
}

export function ContactPicker({
  closeLabel,
  onClose,
  onAdded,
}: {
  /** "Cancel" from Add person, "Skip" in the guide. */
  closeLabel: string;
  onClose: () => void;
  /** After the people were added, with how many. */
  onAdded: (count: number) => void;
}) {
  const phone = onPhone();
  const [loaded, setLoaded] = useState<Loaded>({ status: "loading" });
  const [query, setQuery] = useState("");
  // Who's ticked, by card, with how often to be in touch with each (monthly to start with).
  const [picked, setPicked] = useState<Map<string, number>>(new Map());
  const add = useAction();
  const live = useRef(true);

  const fetchList = () =>
    api.listContacts().then(
      (list) => live.current && setLoaded(list),
      (e) => live.current && setLoaded({ status: "failed", message: errorMessage(e) }),
    );
  useEffect(() => {
    live.current = true;
    fetchList();
    return () => {
      live.current = false;
    };
  }, []);
  const retry = () => {
    setLoaded({ status: "loading" });
    fetchList();
  };

  const contacts = useMemo(() => (loaded.status === "allowed" ? loaded.contacts : []), [loaded]);
  const shown = useMemo(() => contacts.filter((c) => matches(c.name, query)), [contacts, query]);
  const count = picked.size;

  const flip = (id: string, on: boolean) => {
    const next = new Map(picked);
    if (on) next.set(id, DEFAULT_FREQUENCY);
    else next.delete(id);
    setPicked(next);
  };

  const setFrequency = (id: string, days: number) => setPicked(new Map(picked).set(id, days));

  const submit = async () => {
    const people = contacts
      .filter((c) => picked.has(c.id))
      .map((c) => ({ name: c.name, contact: c.id, frequencyDays: picked.get(c.id) ?? DEFAULT_FREQUENCY }));
    if (await add.run(() => api.importContacts(people))) {
      if (live.current) onAdded(people.length);
    }
  };

  const closeButton = (
    <button type="button" className="btn btn-quiet" onClick={onClose}>
      {closeLabel}
    </button>
  );

  if (loaded.status !== "allowed") {
    return (
      <div className="card picker" role="region" aria-label="Contacts">
        <div className="picker-message" role={loaded.status === "loading" ? "status" : "alert"}>
          {loaded.status === "loading" ? (
            <p className="muted">Opening Contacts…</p>
          ) : loaded.status === "denied" ? (
            <>
              <strong>Friendship Reminder can't see your contacts</strong>
              <p className="muted">
                {phone
                  ? "To pick people from Contacts, turn on Contacts for Friendship Reminder in Settings › Apps › Friendship Reminder."
                  : "To pick people from Contacts, turn on Friendship Reminder in System Settings › Privacy & Security › Contacts, then try again."}{" "}
                You can still add people by typing their name.
              </p>
            </>
          ) : (
            <>
              <strong>Couldn't open Contacts</strong>
              <p className="muted">{loaded.message}</p>
            </>
          )}
        </div>
        <div className="form-actions picker-actions">
          {closeButton}
          {loaded.status !== "loading" && (
            <button type="button" className="btn btn-outline" onClick={retry}>
              Try again
            </button>
          )}
        </div>
      </div>
    );
  }

  const free = contacts.filter((c) => !c.added).length;
  return (
    <div className="card picker" role="region" aria-label="Contacts">
      <div className="picker-search">
        <input
          className="input"
          type="search"
          value={query}
          autoFocus={!phone}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search contacts"
          aria-label="Search contacts"
        />
      </div>
      {contacts.length === 0 ? (
        <p className="picker-message muted">There's nobody in Contacts yet.</p>
      ) : shown.length === 0 ? (
        <p className="picker-message muted">Nobody in Contacts matches “{query.trim()}”.</p>
      ) : (
        <ul className="list picker-list" aria-label="Contacts">
          {shown.map((c) => (
            <li key={c.id} className="list-row picker-row" data-added={c.added}>
              <label>
                <input
                  type="checkbox"
                  aria-label={c.added ? `${c.name}, already added` : c.name}
                  checked={c.added || picked.has(c.id)}
                  disabled={c.added}
                  onChange={(e) => flip(c.id, e.target.checked)}
                />
                <Avatar name={c.name} />
                <span className="person-name">{c.name}</span>
                {c.added && (
                  <span className="picker-added" aria-hidden="true">
                    Already added
                  </span>
                )}
              </label>
              {picked.has(c.id) && (
                <select
                  className="input picker-frequency"
                  aria-label={`How often for ${c.name}`}
                  value={picked.get(c.id)}
                  onChange={(e) => setFrequency(c.id, Number(e.target.value))}
                >
                  {FREQUENCIES.map((f) => (
                    <option key={f.days} value={f.days}>
                      {f.label}
                    </option>
                  ))}
                </select>
              )}
            </li>
          ))}
        </ul>
      )}
      <div className="picker-footer">
        <p className="hint picker-hint">
          {free === 0 && contacts.length > 0
            ? "Everyone in Contacts is in your list already."
            : "Pick how often for each person you tick. Each counts as in touch today, so their first reminder comes one round from now."}
        </p>
        {add.error && (
          <p className="error" role="alert">
            {add.error}
          </p>
        )}
        <div className="form-actions picker-actions">
          {closeButton}
          <button type="button" className="btn btn-primary" disabled={count === 0 || add.busy} onClick={submit}>
            {count === 0 ? "Add people" : count === 1 ? "Add 1 person" : `Add ${count} people`}
          </button>
        </div>
      </div>
    </div>
  );
}
