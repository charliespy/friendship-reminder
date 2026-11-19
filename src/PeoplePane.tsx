/** Preferences › People: add, edit, mark done and delete the people Friends reminds you about. */
import { useState, type FormEvent } from "react";
import { api, type Person, type View } from "./api";
import { Avatar } from "./components/Avatar";
import { formatDay, relativeDay } from "./date";
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
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (await run(() => onSubmit({ ...draft, name: draft.name.trim() }, opened))) onCancel();
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
}: {
  person: Person;
  today: string;
  selected: boolean;
  onSelect: (on: boolean) => void;
  onEdit: () => void;
}) {
  const { run, busy, error } = useAction();
  const when = person.due
    ? person.overdue
      ? `Missed last time · aim for ${formatDay(person.nextTarget, today)}`
      : `Due · aim for ${formatDay(person.nextTarget, today)}`
    : `Reminder ${relativeDay(person.notificationDate, today)}, ${formatDay(person.notificationDate, today)}`;
  return (
    <li className="list-row" data-testid="person-row">
      <input
        type="checkbox"
        checked={selected}
        onChange={(e) => onSelect(e.target.checked)}
        aria-label={`Select ${person.name}`}
      />
      <Avatar name={person.name} />
      <div className="person-text">
        <div className="person-name">{person.name}</div>
        <div className="person-meta">
          {frequencyLabel(person.frequencyDays)} · {when}
        </div>
        {error && <p className="error">{error}</p>}
      </div>
      {person.due && <span className="tag">Due</span>}
      <button className="btn btn-outline btn-sm" disabled={busy} onClick={() => run(() => api.markDone(person.id))}>
        Done
      </button>
      <button className="btn btn-quiet btn-sm" onClick={onEdit} aria-label={`Edit ${person.name}`}>
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

  const select = (id: number, on: boolean) => {
    const next = new Set(chosen);
    if (on) next.add(id);
    else next.delete(id);
    setSelected(next);
    setConfirming(false);
  };

  const deleteChosen = async () => {
    if (await del.run(() => api.deletePeople(chosen))) {
      setSelected(new Set());
      setConfirming(false);
      // Close the edit form only if its person was deleted; another person's edit stays open.
      if (editing !== null && chosen.includes(editing)) setEditing(null);
    }
  };

  const editingPerson = view.people.find((p) => p.id === editing);

  return (
    <section aria-labelledby="people-title">
      <header className="pane-header">
        <div>
          <h2 id="people-title">People</h2>
          <p>Who you want to keep up with, and how often.</p>
        </div>
        {!adding && (
          <button className="btn btn-primary" onClick={() => (setAdding(true), setEditing(null))}>
            Add person
          </button>
        )}
      </header>

      {adding && (
        <PersonForm
          initial={{ name: "", frequencyDays: 90, lastConnected: view.today }}
          today={view.today}
          submitLabel="Add person"
          onSubmit={(d) => api.addPerson(d)}
          onCancel={() => setAdding(false)}
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
          onCancel={() => setEditing(null)}
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
              <button className="btn btn-quiet btn-sm" onClick={() => setSelected(new Set())}>
                Deselect all
              </button>
              <button className="btn btn-outline btn-sm" onClick={() => setConfirming(true)}>
                Delete selected…
              </button>
            </>
          )}
          {del.error && <p className="error">{del.error}</p>}
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
            />
          ))}
        </ul>
      )}
    </section>
  );
}
