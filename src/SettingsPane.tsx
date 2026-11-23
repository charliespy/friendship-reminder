/** Preferences › Settings: which days count, notifications, launch at login, and the account. */
import { useEffect, useRef, useState } from "react";
import { api, errorMessage, type DayToggle, type View } from "./api";
import { AccountSection } from "./AccountSection";
import { useAction } from "./useView";

const TOGGLES: { value: DayToggle; title: string; text: string }[] = [
  { value: "weekendsOnly", title: "Weekends only", text: "Aim for a Saturday or Sunday. The reminder comes on the Monday before, so there's time to plan." },
  { value: "weekdaysOnly", title: "Weekdays only", text: "Aim for Monday to Friday, like a lunch or a call." },
  { value: "allDays", title: "Any day", text: "Aim for whatever day comes up. Weekend days still get a Monday heads-up." },
];

export function SettingsPane({ view }: { view: View }) {
  const toggle = useAction();
  const notify = useAction();
  const [login, setLogin] = useState<boolean | null>(null);
  const [loginError, setLoginError] = useState<string | null>(null);
  const [loginBusy, setLoginBusy] = useState(false);

  // Bumped by every read and change, so a read that finishes after a newer click is dropped.
  const loginSeq = useRef(0);

  // Read on mount, and again whenever the window comes back: the pane stays mounted while
  // Preferences is hidden, and the setting can change in System Settings meanwhile.
  useEffect(() => {
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
  }, []);

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
          <p>Which days count for meeting up, and how Friends reminds you.</p>
        </div>
      </header>

      <fieldset className="setting-group plain-fieldset">
        <legend className="section-label">Days to aim for</legend>
        <div className="choice-list">
          {TOGGLES.map((t) => (
            <label className="choice" key={t.value}>
              <input
                type="radio"
                name="dayToggle"
                value={t.value}
                checked={view.settings.dayToggle === t.value}
                disabled={toggle.busy}
                onChange={() => toggle.run(() => api.setDayToggle(t.value))}
              />
              <span>
                <span className="choice-title">{t.title}</span>
                <br />
                <span className="choice-text">{t.text}</span>
              </span>
            </label>
          ))}
        </div>
        <p className="hint">
          Changing this reschedules everyone from the last time you were in touch.
        </p>
        {toggle.error && <p className="error" role="alert">{toggle.error}</p>}
      </fieldset>

      <div className="setting-group">
        <h3 className="section-label">Reminders</h3>
        <div className="choice-list">
          <label className="choice">
            <input
              type="checkbox"
              checked={view.settings.notifications}
              disabled={notify.busy}
              onChange={(e) => notify.run(() => api.setNotifications(e.target.checked))}
            />
            <span>
              <span className="choice-title">Send a notification</span>
              <br />
              <span className="choice-text">
                When someone's reminder comes up. They also show in the menu bar either way.
              </span>
            </span>
          </label>
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
              <span className="choice-text">Friends starts in the menu bar when you log in.</span>
            </span>
          </label>
        </div>
        {notify.error && <p className="error" role="alert">{notify.error}</p>}
        {loginError && <p className="error" role="alert">{loginError}</p>}
      </div>

      <AccountSection />
    </section>
  );
}
