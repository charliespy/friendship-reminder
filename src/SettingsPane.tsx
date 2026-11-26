/**
 * Preferences › Settings: which days count, notifications, launch at login, and the account. On the
 * iPhone (`phone`) there's no launch at login, and due people show in the Today tab.
 */
import { useEffect, useRef, useState } from "react";
import { api, errorMessage, type DayToggle, type View, type Weekday } from "./api";
import { AccountSection } from "./AccountSection";
import type { NotificationAccessState } from "./useNotificationAccess";
import { useAction } from "./useView";

const TOGGLES: { value: DayToggle; title: string; text: string }[] = [
  { value: "weekendsOnly", title: "Weekends only", text: "Aim for a Saturday or Sunday. The reminder comes on the Monday before, so there's time to plan." },
  { value: "weekdaysOnly", title: "Weekdays only", text: "Aim for Monday to Friday, like a lunch or a call." },
  { value: "custom", title: "Custom day", text: "Aim only for the days you're free. A missed day moves to the next one you picked. Weekend days still get a Monday heads-up." },
];

const WEEK: { day: Weekday; short: string; name: string }[] = [
  { day: "mon", short: "M", name: "Monday" },
  { day: "tue", short: "T", name: "Tuesday" },
  { day: "wed", short: "W", name: "Wednesday" },
  { day: "thu", short: "T", name: "Thursday" },
  { day: "fri", short: "F", name: "Friday" },
  { day: "sat", short: "S", name: "Saturday" },
  { day: "sun", short: "S", name: "Sunday" },
];

/** The days picked under "Custom day": one button per day, on while picked. Changes wait for Save. */
function DayPicker({
  days,
  disabled,
  onChange,
}: {
  days: Weekday[];
  disabled: boolean;
  onChange: (days: Weekday[]) => void;
}) {
  const picked = new Set(days);
  const flip = (day: Weekday) =>
    onChange(WEEK.map((w) => w.day).filter((d) => (d === day ? !picked.has(d) : picked.has(d))));
  return (
    <div className="day-picker-row">
      <div className="day-picker" role="group" aria-label="Days you're free">
        {WEEK.map((w) => {
          const on = picked.has(w.day);
          return (
            <button
              key={w.day}
              type="button"
              className="day-chip"
              aria-pressed={on}
              aria-label={w.name}
              title={w.name}
              // The last day picked stays: at least one is needed.
              disabled={disabled || (on && picked.size === 1)}
              onClick={() => flip(w.day)}
            >
              {w.short}
            </button>
          );
        })}
      </div>
    </div>
  );
}

const sameDays = (a: Weekday[], b: Weekday[]) => a.length === b.length && a.every((d, i) => d === b[i]);

/**
 * Days to aim for. It reschedules everyone, so a change is a draft until Save (Discard puts the
 * saved setting back); the days only count under Custom day.
 */
function DaySetting({ view }: { view: View }) {
  const save = useAction();
  const saved = view.settings;
  const [draft, setDraft] = useState<{ toggle: DayToggle; days: Weekday[] } | null>(null);
  const [justSaved, setJustSaved] = useState(false);
  const toggle = draft?.toggle ?? saved.dayToggle;
  const days = draft?.days ?? saved.customDays;
  const changed =
    toggle !== saved.dayToggle || (toggle === "custom" && !sameDays(days, saved.customDays));
  // Once the saved setting matches the draft (saved, or changed back by hand), the draft is done.
  // Set while rendering, as React suggests for state that follows props.
  // (The whole draft must match: days edited under Custom day stay while another choice is tried.)
  if (draft && draft.toggle === saved.dayToggle && sameDays(draft.days, saved.customDays)) setDraft(null);
  const discard = () => {
    setDraft(null);
    setJustSaved(false);
  };
  const edit = (next: { toggle?: DayToggle; days?: Weekday[] }) => {
    setJustSaved(false);
    setDraft({ toggle: next.toggle ?? toggle, days: next.days ?? days });
  };
  const onSave = async () => {
    const ok = await save.run(() => api.setDayToggle(toggle, toggle === "custom" ? days : undefined));
    // The draft stays until the new view shows it saved (the view can come after the command
    // returns), so the choice never flicks back to the old one meanwhile.
    if (ok) setJustSaved(true);
  };

  return (
    <fieldset className="setting-group plain-fieldset">
      <legend className="section-label">Days to aim for</legend>
      <div className="choice-list">
        {TOGGLES.map((t) => [
          <label className="choice" key={t.value}>
            <input
              type="radio"
              name="dayToggle"
              value={t.value}
              checked={toggle === t.value}
              disabled={save.busy}
              onChange={() => edit({ toggle: t.value })}
            />
            <span>
              <span className="choice-title">{t.title}</span>
              <br />
              <span className="choice-text">{t.text}</span>
            </span>
          </label>,
          t.value === "custom" && toggle === "custom" && (
            <DayPicker key="days" days={days} disabled={save.busy} onChange={(d) => edit({ days: d })} />
          ),
        ])}
      </div>
      <p className="hint">Saving reschedules everyone from the last time you were in touch.</p>
      {save.error && (
        <p className="error" role="alert">
          {save.error}
        </p>
      )}
      <div className="form-actions settings-actions">
        <p className="settings-status" role="status">
          {changed ? "Not saved yet." : justSaved ? "Saved." : ""}
        </p>
        {changed && (
          <button type="button" className="btn btn-quiet" disabled={save.busy} onClick={discard}>
            Discard
          </button>
        )}
        <button type="button" className="btn btn-primary" disabled={!changed || save.busy} onClick={onSave}>
          Save
        </button>
      </div>
    </fieldset>
  );
}

export function SettingsPane({
  view,
  phone = false,
  access,
}: {
  view: View;
  phone?: boolean;
  /** The iPhone's notification permission (the Mac's needs nothing here). */
  access?: NotificationAccessState;
}) {
  const notify = useAction();
  // iOS hasn't answered (or its answer couldn't be read): turning reminders on asks it too.
  const mayAsk = access?.access === "notAsked" || access?.access === "unknown";
  const setNotifications = (on: boolean) =>
    notify.run(async () => {
      await api.setNotifications(on);
      if (on && mayAsk) await access?.allow();
    });
  const [login, setLogin] = useState<boolean | null>(null);
  const [loginError, setLoginError] = useState<string | null>(null);
  const [loginBusy, setLoginBusy] = useState(false);

  // Bumped by every read and change, so a read that finishes after a newer click is dropped.
  const loginSeq = useRef(0);

  // Read on mount, and again whenever the window comes back: the pane stays mounted while
  // Preferences is hidden, and the setting can change in System Settings meanwhile.
  useEffect(() => {
    if (phone) return;
    const load = () => {
      const seq = ++loginSeq.current;
      api.getLaunchAtLogin().then(
        (on) => {
          if (seq !== loginSeq.current) return;
          setLogin(on);
          setLoginError(null);
        },
        (e) => {
          if (seq === loginSeq.current) setLoginError(errorMessage(e));
        },
      );
    };
    const onVisible = () => {
      if (document.visibilityState === "visible") load();
    };
    load();
    window.addEventListener("focus", load);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.removeEventListener("focus", load);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [phone]);

  // One change at a time, so the box always shows the answer to the last click.
  const changeLogin = async (on: boolean) => {
    loginSeq.current++;
    setLoginError(null);
    setLoginBusy(true);
    try {
      const now = await api.setLaunchAtLogin(on);
      // A read that started during the change may have seen the old value: drop it.
      loginSeq.current++;
      setLogin(now);
    } catch (e) {
      loginSeq.current++;
      setLoginError(errorMessage(e));
    } finally {
      setLoginBusy(false);
    }
  };

  return (
    <section aria-labelledby="settings-title">
      <header className="pane-header">
        <div>
          <h2 id="settings-title">Settings</h2>
          <p>Which days count for meeting up, and how Friendship Reminder reminds you.</p>
        </div>
      </header>

      <DaySetting view={view} />

      <div className="setting-group">
        <h3 className="section-label">Reminders</h3>
        <div className="choice-list">
          <label className="choice">
            <input
              type="checkbox"
              checked={view.settings.notifications}
              disabled={notify.busy}
              onChange={(e) => setNotifications(e.target.checked)}
            />
            <span>
              <span className="choice-title">Send a notification</span>
              <br />
              <span className="choice-text">
                {phone
                  ? "On the morning someone's reminder comes up. They also show in Today either way."
                  : "When someone's reminder comes up. They also show in the menu bar either way."}
              </span>
            </span>
          </label>
          {view.settings.notifications && mayAsk && (
            <div className="choice-note">
              <p className="hint">
                {access?.access === "notAsked"
                  ? "iOS hasn't been asked to let Friendship Reminder send notifications yet."
                  : "Couldn't check whether iOS lets Friendship Reminder send notifications."}
              </p>
              <button type="button" className="btn btn-outline btn-sm" disabled={access?.busy} onClick={access?.allow}>
                Allow notifications
              </button>
            </div>
          )}
          {view.settings.notifications && access?.access === "denied" && (
            <p className="hint choice-note">
              Notifications are off for Friendship Reminder. To get them, turn them on in Settings › Apps › Friendship
              Reminder › Notifications.
            </p>
          )}
          {!phone && (
            <label className="choice">
              <input
                type="checkbox"
                checked={login ?? false}
                disabled={loginBusy || (login === null && !loginError)}
                onChange={(e) => changeLogin(e.target.checked)}
              />
              <span>
                <span className="choice-title">Open at login</span>
                <br />
                <span className="choice-text">Friendship Reminder starts in the menu bar when you log in.</span>
              </span>
            </label>
          )}
        </div>
        {notify.error && <p className="error" role="alert">{notify.error}</p>}
        {access?.error && <p className="error" role="alert">{access.error}</p>}
        {loginError && <p className="error" role="alert">{loginError}</p>}
      </div>

      <AccountSection phone={phone} />
    </section>
  );
}
