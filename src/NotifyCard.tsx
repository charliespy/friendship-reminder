/**
 * On iPhone, the app asks iOS to send notifications only once the user taps to turn reminders on:
 * this card, on Today, says what they are first. It shows once the list has people (and reminders
 * are on in Settings), and "Not now" hides it until the app opens again.
 */
import { useState } from "react";
import type { View } from "./api";
import { useFocusWhenGone } from "./useFocusWhenGone";
import type { NotificationAccessState } from "./useNotificationAccess";

/** `afterClose`: where focus goes when the card goes away with its focused button. */
export function NotifyCard({
  view,
  notify,
  afterClose,
}: {
  view: View;
  notify: NotificationAccessState;
  afterClose: () => HTMLElement | null;
}) {
  const [later, setLater] = useState(false);
  const shown = !later && notify.access === "notAsked" && view.settings.notifications && view.people.length > 0;
  const focus = useFocusWhenGone(shown, afterClose);
  if (!shown) return null;
  return (
    <section className="card notify-card" aria-labelledby="notify-title" {...focus}>
      <h3 id="notify-title">Get reminded on the day</h3>
      <p className="muted">
        Friendship Reminder can send a notification on the morning it's time to reach out to someone. Next, iOS asks
        whether to allow it.
      </p>
      {notify.error && (
        <p className="error" role="alert">
          {notify.error}
        </p>
      )}
      <div className="form-actions">
        <button type="button" className="btn btn-quiet" disabled={notify.busy} onClick={() => setLater(true)}>
          Not now
        </button>
        <button type="button" className="btn btn-primary" disabled={notify.busy} onClick={notify.allow}>
          Turn on reminders
        </button>
      </div>
    </section>
  );
}
