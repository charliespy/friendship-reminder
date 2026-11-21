/** Preferences › People: add, edit, mark done and delete the people Friends reminds you about. */
import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, type Person, type View } from "./api";
import { Avatar } from "./components/Avatar";
import { aimDay, formatDay, relativeDay } from "./date";
import { FREQUENCIES, frequencyLabel } from "./format";
import { useAction } from "./useView";

interface Draft {
  name: string;
  frequencyDays: number;
  lastConnected: string;
}

function PersonForm({
  initial,
  today,
  submitLabel,
  onSubmit,
  onCancel,
}: {
  initial: Draft;
  today: string;
  submitLabel: string;
  /** `opened` is the draft as the form first showed it. */
  onSubmit: (d: Draft, opened: Draft) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [opened] = useState(initial);
  const [draft, setDraft] = useState(initial);
  const { run, busy, error } = useAction();
  // A save can finish after the form was closed another way (e.g. Edit on someone else); then
  // there's nothing left to close, and no focus to send back.
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if ((await run(() => onSubmit({ ...draft, name: draft.name.trim() }, opened))) && mounted.current) onCancel();
  };
  return (
    <form className="card form-card" onSubmit={submit} aria-label={submitLabel} noValidate>
      <div className="form-row">
        <label className="field grow">
          <span>Name</span>
          <input
            className="input"
            value={draft.name}
            maxLength={100}
            autoFocus
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            placeholder="Who do you want to keep up with?"
          />
        </label>
        <label className="field">
          <span>How often</span>
          <select
            className="input"
            value={draft.frequencyDays}
            onChange={(e) => setDraft({ ...draft, frequencyDays: Number(e.target.value) })}
          >
            {FREQUENCIES.map((f) => (
              <option key={f.days} value={f.days}>
                {f.label}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>Last in touch</span>
          <input
            className="input"
            type="date"
            value={draft.lastConnected}
            max={today}
            min="1900-01-01"
            onChange={(e) => setDraft({ ...draft, lastConnected: e.target.value })}
          />
        </label>
      </div>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="form-actions">
        <button type="button" className="btn btn-quiet" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={busy || !draft.name.trim() || !draft.lastConnected}>
          {submitLabel}
        </button>
      </div>
    </form>
  );
}

function PersonRow({
  person,
  today,
  selected,
  onSelect,
  onEdit,
  onDoneStart,
  onDoneEnd,
  selectRef,
  doneRef,
  editRef,
}: {
  person: Person;
  today: string;
  selected: boolean;
  onSelect: (on: boolean) => void;
  onEdit: () => void;
  /** Called when Done is pressed, before the command runs. */
  onDoneStart: () => void;
  /** Called when the Done command has finished, with whether it worked. */
  onDoneEnd: (ok: boolean) => void;
  selectRef: (el: HTMLInputElement | null) => void;
  doneRef: (el: HTMLButtonElement | null) => void;
  editRef: (el: HTMLButtonElement | null) => void;
}) {
  const { run, busy, error } = useAction();
  const when = person.due
    ? person.overdue
      ? `Missed last time · aim for ${aimDay(person.nextTarget, today)}`
      : `Due · aim for ${aimDay(person.nextTarget, today)}`
    : `Reminder ${relativeDay(person.notificationDate, today)}, ${formatDay(person.notificationDate, today)}`;
  return (
    <li className="list-row" data-testid="person-row" data-overdue={person.overdue}>
      <input
        ref={selectRef}
        type="checkbox"
        checked={selected}
        onChange={(e) => onSelect(e.target.checked)}
        aria-label={`Select ${person.name}`}
      />
      <Avatar name={person.name} />
      <div className="person-text">
        <div className="person-name">
          {person.name}
          {person.due && <span className="tag">Due</span>}
        </div>
        <div className="person-meta">
          {frequencyLabel(person.frequencyDays)} · {when}
        </div>
        {error && <p className="error" role="alert">{error}</p>}
      </div>
      <button
        ref={doneRef}
        className="btn btn-done btn-sm"
        disabled={busy}
        onClick={async () => {
          onDoneStart();
          onDoneEnd(await run(() => api.markDone(person.id)));
        }}
        aria-label={`Mark ${person.name} as done`}
      >
        Done
      </button>
      <button ref={editRef} className="btn btn-quiet btn-sm" onClick={onEdit} aria-label={`Edit ${person.name}`}>
        Edit
      </button>
    </li>
  );
}

export function PeoplePane({ view }: { view: View }) {
  const [adding, setAdding] = useState(false);
  const [editing, setEditing] = useState<number | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const del = useAction();

  // Forget selections of people who are gone.
  const present = new Set(view.people.map((p) => p.id));
  const chosen = [...selected].filter((id) => present.has(id));

  // When a form closes, focus goes back to the button that opened it (Add person, or the person's
  // Edit), so keyboard and screen reader users don't land back at the top of the window. After a
  // delete, it goes to the checkbox of the person who takes the first deleted person's place (or
  // the one before, if the deleted people were at the end), or to Add person when nobody is left.
  // After a Done, the list re-sorts and moves the person's row, which drops focus: it goes back to
  // their Done button in its new place. After Deselect all, the selection bar goes away with the
  // focused button: focus goes to the checkbox of the first person who was selected.
  const section = useRef<HTMLElement>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const editButtons = useRef(new Map<number, HTMLButtonElement>());
  const checkboxes = useRef(new Map<number, HTMLInputElement>());
  const doneButtons = useRef(new Map<number, HTMLButtonElement>());
  const returnFocus = useRef<
    | "add"
    | number
    | { deleted: number[]; next: number | null }
    | { done: number; before: View; finished: boolean }
    | { deselected: number }
    | null
  >(null);
  // Re-renders the pane when a Done command finishes (its busy state lives in the row).
  const [, setDoneFinished] = useState(0);
  useEffect(() => {
    const target = returnFocus.current;
    if (target === null) return;
    // The new view can arrive after the command returns: wait until it no longer shows the deleted
    // people. After a Done, the new view can come before or after the command returns (the macOS
    // app sends it first): wait for both, so the Done button is enabled again, and for a newer view
    // than the one Done was pressed in that shows the person as done today, is gone, or is a new day
    // (then the target is used up either way, so it can't take focus later).
    if (typeof target === "object" && "deleted" in target && target.deleted.some((id) => present.has(id))) return;
    if (typeof target === "object" && "done" in target) {
      const person = view.people.find((p) => p.id === target.done);
      const newDay = view.today !== target.before.today;
      if (!target.finished || view === target.before || (person && person.lastConnected !== view.today && !newDay)) {
        return;
      }
    }
    returnFocus.current = null;
    // Only if focus was lost; never take it from somewhere the user moved it.
    if (document.activeElement && document.activeElement !== document.body) return;
    if (typeof target === "object" && "done" in target) {
      doneButtons.current.get(target.done)?.focus();
    } else if (typeof target === "object" && "deselected" in target) {
      checkboxes.current.get(target.deselected)?.focus();
    } else if (typeof target === "object") {
      const next = target.next === null ? undefined : checkboxes.current.get(target.next);
      // Add person is hidden while its form is open; then the form's first field takes focus.
      (next ?? addButton.current ?? section.current?.querySelector<HTMLElement>("form input"))?.focus();
    } else {
      (target === "add" ? addButton.current : editButtons.current.get(target))?.focus();
    }
  });

  const select = (id: number, on: boolean) => {
    const next = new Set(chosen);
    if (on) next.add(id);
    else next.delete(id);
    setSelected(next);
    setConfirming(false);
    del.setError(null);
  };

  const deselectAll = () => {
    // The selection bar, and the Deselect all button in it, are about to go.
    const first = view.people.find((p) => chosen.includes(p.id));
    if (first) returnFocus.current = { deselected: first.id };
    setSelected(new Set());
    del.setError(null);
  };

  const deleteChosen = async () => {
    // Who takes the first deleted person's place: the next person who stays, else the one before.
    const people = view.people;
    const first = people.findIndex((p) => chosen.includes(p.id));
    const stays = (p: Person) => !chosen.includes(p.id);
    const next = people.slice(first).find(stays) ?? people.slice(0, first).findLast(stays);
    if (await del.run(() => api.deletePeople(chosen))) {
      // The selection bar, and the Delete button in it, are about to go.
      returnFocus.current = { deleted: chosen, next: next?.id ?? null };
      setSelected(new Set());
      setConfirming(false);
      // Close the edit form only if its person was deleted; another person's edit stays open.
      if (editing !== null && chosen.includes(editing)) setEditing(null);
    }
  };

  const editingPerson = view.people.find((p) => p.id === editing);

  return (
    <section ref={section} aria-labelledby="people-title">
      <header className="pane-header">
        <div>
          <h2 id="people-title">People</h2>
          <p>Who you want to keep up with, and how often.</p>
        </div>
        {!adding && (
          <button ref={addButton} className="btn btn-primary" onClick={() => (setAdding(true), setEditing(null))}>
            Add person
          </button>
        )}
      </header>

      {adding && (
        <PersonForm
          initial={{ name: "", frequencyDays: 30, lastConnected: view.today }}
          today={view.today}
          submitLabel="Add person"
          onSubmit={(d) => api.addPerson(d)}
          onCancel={() => ((returnFocus.current = "add"), setAdding(false))}
        />
      )}

      {editingPerson && (
        <PersonForm
          key={editingPerson.id}
          initial={{
            name: editingPerson.name,
            frequencyDays: editingPerson.frequencyDays,
            lastConnected: editingPerson.lastConnected,
          }}
          today={view.today}
          submitLabel="Save changes"
          onSubmit={(d, opened) =>
            api.updatePerson({
              id: editingPerson.id,
              name: d.name,
              frequencyDays: d.frequencyDays,
              ...(d.lastConnected !== opened.lastConnected && { lastConnected: d.lastConnected }),
            })
          }
          onCancel={() => ((returnFocus.current = editingPerson.id), setEditing(null))}
        />
      )}

      {chosen.length > 0 && (
        <div className="selection-bar" role="region" aria-label="Selected people">
          <span className="count">{chosen.length} selected</span>
          {confirming ? (
            <>
              <span className="muted">Delete {chosen.length === 1 ? "this person" : "these people"}?</span>
              <button className="btn btn-quiet btn-sm" onClick={() => setConfirming(false)}>
                Keep
              </button>
              <button className="btn btn-danger btn-sm" disabled={del.busy} onClick={deleteChosen}>
                Delete
              </button>
            </>
          ) : (
            <>
              <button className="btn btn-quiet btn-sm" onClick={deselectAll}>
                Deselect all
              </button>
              <button className="btn btn-outline btn-sm" onClick={() => setConfirming(true)}>
                Delete selected…
              </button>
            </>
          )}
          {del.error && <p className="error" role="alert">{del.error}</p>}
        </div>
      )}

      {view.people.length === 0 ? (
        !adding && (
          <div className="card empty">
            <strong>Nobody yet</strong>
            Add someone, and Friends will remind you when it's time to reach out.
          </div>
        )
      ) : (
        <ul className="list card">
          {view.people.map((p) => (
            <PersonRow
              key={p.id}
              person={p}
              today={view.today}
              selected={chosen.includes(p.id)}
              onSelect={(on) => select(p.id, on)}
              onEdit={() => (setEditing(p.id), setAdding(false))}
              // `view` here is the one Done was pressed in.
              onDoneStart={() => (returnFocus.current = { done: p.id, before: view, finished: false })}
              onDoneEnd={(ok) => {
                const target = returnFocus.current;
                if (typeof target !== "object" || target === null || !("done" in target) || target.done !== p.id) return;
                if (ok) {
                  target.finished = true;
                  setDoneFinished((n) => n + 1);
                } else {
                  returnFocus.current = null;
                }
              }}
              selectRef={(el) => {
                if (el) checkboxes.current.set(p.id, el);
                else checkboxes.current.delete(p.id);
              }}
              doneRef={(el) => {
                if (el) doneButtons.current.set(p.id, el);
                else doneButtons.current.delete(p.id);
              }}
              editRef={(el) => {
                if (el) editButtons.current.set(p.id, el);
                else editButtons.current.delete(p.id);
              }}
            />
          ))}
        </ul>
      )}
    </section>
  );
}
