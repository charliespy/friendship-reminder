/**
 * Picking people from Contacts: search the cards, tick one or several, pick how often for each, and
 * add them.
 * Used from Add person and from the first-run guide. The app reads only names (see SECURITY.md).
 *
 * - Before Apple asks for access the first time, the picker says why the app wants it.
 * - People chosen in Apple's own sheets (sharing only some contacts, on iPhone) were picked there
 *   already: they go straight to "how often", with no second list to tick.
 * - With only some contacts shared, "Share more contacts…" opens the app's page in iOS Settings;
 *   whoever is shared there goes straight to "how often" on the way back.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { api, errorMessage, type ContactList } from "./api";
import { Avatar } from "./components/Avatar";
import { FREQUENCIES } from "./format";
import { useFocusWhenGone } from "./useFocusWhenGone";
import { useAction } from "./useView";

type Loaded = { status: "loading"; text?: string } | ContactList | { status: "failed"; message: string };

/**
 * Letters that are their own letter, not a plain one plus an accent mark, so dropping marks leaves
 * them as they are: spelled the way an English keyboard would type them. Also Catalan's middle dot
 * ("l·l"), which a keyboard leaves out, and curly apostrophes, which it types straight.
 */
const PLAIN: Record<string, string> = {
  ı: "i",
  ł: "l",
  ŀ: "l",
  ø: "o",
  đ: "d",
  ð: "d",
  þ: "th",
  ħ: "h",
  ŧ: "t",
  ŋ: "n",
  ß: "ss",
  æ: "ae",
  œ: "oe",
  ə: "a",
  ĳ: "ij",
  ſ: "s",
  "·": "",
  "’": "'",
  "‘": "'",
  "ʼ": "'",
};

/** How a search matches a name: any order of words, ignoring case and accents. */
function fold(text: string): string {
  const unmarked = text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
  return Array.from(unmarked, (c) => PLAIN[c] ?? c).join("");
}

export function matches(name: string, query: string): boolean {
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
  // The people just chosen in Apple's sheet, shown alone for "how often" (null: the whole list).
  const [chosen, setChosen] = useState<string[] | null>(null);
  const add = useAction();
  const live = useRef(true);

  // The ticks as of this render, for answers that arrive later.
  const pickedNow = useRef(picked);
  useEffect(() => {
    pickedNow.current = picked;
  }, [picked]);

  const take = (list: ContactList) => {
    if (!live.current) return;
    if (list.status === "allowed") {
      // Anyone ticked who can still be added keeps how often they had (someone un-shared in
      // Settings drops out); the people just chosen join them.
      const open = new Set(list.contacts.filter((c) => !c.added).map((c) => c.id));
      const next = new Map([...pickedNow.current].filter(([id]) => open.has(id)));
      for (const id of list.picked) if (!next.has(id)) next.set(id, DEFAULT_FREQUENCY);
      setPicked(next);
      // How often shows everyone ticked, so what it shows is what Add adds.
      setChosen(list.picked.length > 0 ? [...next.keys()] : null);
    }
    setLoaded(list);
  };
  const fail = (e: unknown) => live.current && setLoaded({ status: "failed", message: errorMessage(e) });
  /** The list; `ask`: Apple asks for access now (after the explanation). */
  const load = (ask: boolean) => api.listContacts(ask).then(take, fail);
  useEffect(() => {
    live.current = true;
    // Never asks (that's Continue's job), so React running effects twice in development is harmless.
    load(false);
    return () => {
      live.current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const again = (ask: boolean) => {
    setLoaded({ status: "loading" });
    load(ask);
  };
  const shareMore = () => {
    setLoaded({ status: "loading", text: "Share more in Settings, then come back here." });
    api.shareMoreContacts().then(take, fail);
  };

  const contacts = useMemo(() => (loaded.status === "allowed" ? loaded.contacts : []), [loaded]);
  const shown = useMemo(() => {
    if (!chosen) return contacts.filter((c) => matches(c.name, query));
    const ids = new Set(chosen);
    return contacts.filter((c) => ids.has(c.id));
  }, [contacts, chosen, query]);
  const count = picked.size;

  // Continue, Try again and Share more give way to "Opening Contacts…", and that to the list or a
  // message, in the same place: focus goes to what each shows now, not to the page itself. On iPhone
  // that's never the search field, which would bring up the keyboard.
  const region = useRef<HTMLDivElement>(null);
  const swap = useFocusWhenGone(chosen ? "chosen" : loaded.status, () => {
    // In this order: "How often" for someone just chosen, else someone to tick, else the line that says
    // there's nobody left (never Skip or Cancel, where a second Enter would close the picker).
    for (const where of ["[data-focus]", "select", 'input[type="checkbox"]:not(:disabled)', ".picker-hint"]) {
      const found = region.current?.querySelector<HTMLElement>(where);
      if (found) return found;
    }
    return region.current?.querySelector<HTMLElement>("button:not(:disabled)");
  });

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

  if (loaded.status === "notAsked") {
    return (
      <div className="card picker" role="region" aria-label="Contacts" ref={region} {...swap}>
        <div className="picker-message picker-explain">
          <strong>Add people from your Contacts</strong>
          <p className="muted">
            Friendship Reminder reads only names. On the card of each person you add, it puts a link back to them in the
            app.
          </p>
          <p className="muted">
            {phone
              ? "Next, iOS asks whether to share your contacts. You can share all of them, or choose just the people you want."
              : "Next, macOS asks whether Friendship Reminder may use your contacts."}
          </p>
        </div>
        <div className="form-actions picker-actions">
          {closeButton}
          <button type="button" className="btn btn-primary" autoFocus={!phone} data-focus onClick={() => again(true)}>
            Continue
          </button>
        </div>
      </div>
    );
  }

  if (loaded.status !== "allowed") {
    return (
      <div className="card picker" role="region" aria-label="Contacts" ref={region} {...swap}>
        {/* While it loads, focus waits on this line, not on Skip or Cancel (a second Enter there
            would close the picker); a new message for each status, so focus moves on with it. */}
        <div
          key={loaded.status}
          className="picker-message"
          role={loaded.status === "loading" ? "status" : "alert"}
          tabIndex={loaded.status === "loading" ? -1 : undefined}
          data-focus={loaded.status === "loading" || undefined}
        >
          {loaded.status === "loading" ? (
            <p className="muted">{loaded.text ?? "Opening Contacts…"}</p>
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
            <button type="button" className="btn btn-outline" data-focus onClick={() => again(false)}>
              Try again
            </button>
          )}
        </div>
      </div>
    );
  }

  const free = contacts.filter((c) => !c.added).length;
  return (
    <div className="card picker" role="region" aria-label="Contacts" ref={region} {...swap}>
      {chosen ? (
        <div className="picker-heading">
          <h2>How often?</h2>
          <p className="muted">Pick how often to be in touch with each person you chose.</p>
        </div>
      ) : (
        <div className="picker-search">
          <input
            className="input"
            type="search"
            value={query}
            autoFocus={!phone}
            data-focus={!phone || undefined}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search contacts"
            aria-label="Search contacts"
          />
        </div>
      )}
      {contacts.length === 0 ? (
        <p className="picker-message muted" tabIndex={-1} data-focus>
          {loaded.limited ? "You haven't shared anyone from Contacts yet." : "There's nobody in Contacts yet."}
        </p>
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
      {!chosen && loaded.limited && (
        <div className="picker-share">
          <p className="hint">
            You shared only some contacts. To add others, tap below, then Contacts › Edit Selected Contacts in Settings.
            They show up here when you come back.
          </p>
          <button type="button" className="btn btn-outline btn-sm" onClick={shareMore}>
            Share more contacts…
          </button>
        </div>
      )}
      <div className="picker-footer">
        <p className="hint picker-hint" tabIndex={-1}>
          {chosen
            ? "Each counts as in touch today, so their first reminder comes one round from now."
            : free === 0 && contacts.length > 0
              ? loaded.limited
                ? "Everyone you've shared from Contacts is in your list already."
                : "Everyone in Contacts is in your list already."
              : "Pick how often for each person you tick. Each counts as in touch today, so their first reminder comes one round from now."}
        </p>
        {chosen && (
          <button type="button" className="btn btn-quiet picker-more" onClick={() => setChosen(null)}>
            Choose others…
          </button>
        )}
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
