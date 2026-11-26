/**
 * The app's commands, as the Rust side (friends-core's `App::call`) answers them. Every command returns
 * the whole view, so the UI never works out a schedule itself.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** A calendar date, "YYYY-MM-DD". */
export type IsoDate = string;

export type DayToggle = "custom" | "weekdaysOnly" | "weekendsOnly";

/** A day of the week, as the Rust side names it. */
export type Weekday = "mon" | "tue" | "wed" | "thu" | "fri" | "sat" | "sun";

export interface Person {
  id: number;
  name: string;
  frequencyDays: number;
  lastConnected: IsoDate;
  nextTarget: IsoDate;
  notificationDate: IsoDate;
  /** A target went by without being marked done. */
  overdue: boolean;
  /** The reminder is showing now. */
  due: boolean;
  /** Done's "On the day suggested" answer: the day the app suggested, when it was before today. */
  suggestedEarlier: IsoDate | null;
  /** Today is the day suggested. */
  suggestedToday: boolean;
  /** The earliest day Done takes. */
  earliestDone: IsoDate;
}

export interface Blocked {
  id: number;
  start: IsoDate;
  end: IsoDate;
  label: string;
}

export interface Settings {
  dayToggle: DayToggle;
  /** The days picked under "Custom day", Monday first. */
  customDays: Weekday[];
  notifications: boolean;
}

/** The first-run guide's steps that are behind you, on this device. */
export interface Guide {
  /** The sign-in page: signed in, or "Not now". */
  signIn: boolean;
  /** Adding people from Contacts: added, skipped, or the list already had people. */
  import: boolean;
}

export type GuideStep = keyof Guide;

export interface View {
  today: IsoDate;
  people: Person[];
  blocked: Blocked[];
  settings: Settings;
  unreadableFile: string | null;
  guide: Guide;
}

/** A card in the address book, as the picker shows it. */
export interface ContactChoice {
  /** This device's id for the card. */
  id: string;
  name: string;
  /** The person is in the list already. */
  added: boolean;
}

/** The address book's cards, or why the app can't see them. */
export type ContactList =
  | {
      status: "allowed";
      contacts: ContactChoice[];
      /** Only some contacts are shared with the app (iPhone). */
      limited: boolean;
      /** The cards just chosen in Apple's own sheets: they go straight to "how often". */
      picked: string[];
    }
  /** Not asked yet: the app explains before Apple asks. */
  | { status: "notAsked" }
  | { status: "denied" };

/** Whether the app may send notifications. Only the iPhone can answer "notAsked": it asks when the user turns reminders on. */
export type NotificationAccess = "allowed" | "notAsked" | "denied";

export interface ApiError {
  kind: "invalid" | "storage" | "badCall";
  message: string;
}

export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e && typeof e.message === "string") return e.message;
  if (typeof e === "string") return e;
  return "Something went wrong.";
}

/** How syncing with the account is going. */
export interface SyncStatus {
  /** A sync is running now. */
  syncing: boolean;
  /** When the last sync finished, in milliseconds since 1970. */
  lastSynced: number | null;
  /** Why the last sync failed, if it did. */
  error: string | null;
}

/** The Friendship Reminder account, as the Account section shows it. */
export type Account =
  | { status: "unavailable"; reason: string }
  | { status: "signedOut"; warning: string | null }
  | { status: "signedIn"; email: string | null; warning: string | null; sync?: SyncStatus | null };

/** Sent to every window after any change, with the new view. */
export const VIEW_CHANGED = "view-changed";

/** Sent to the windows when a link from a contact card opened the app on someone. */
export const SHOW_PERSON = "show-person";

/** Sent to every window when the account changes, with the new account. */
export const ACCOUNT_CHANGED = "account-changed";

function call(cmd: string, args: Record<string, unknown> = {}): Promise<View> {
  return invoke<View>("call", { cmd, args });
}

export const api = {
  getView: () => call("get_view"),
  addPerson: (p: { name: string; frequencyDays: number; lastConnected: IsoDate }) => call("add_person", p),
  /** Leave out lastConnected when it wasn't changed, so a "Done" made meanwhile is kept. */
  updatePerson: (p: { id: number; name: string; frequencyDays: number; lastConnected?: IsoDate }) =>
    call("update_person", p),
  deletePeople: (ids: number[]) => call("delete_people", { ids }),
  /** `on` is the day you were in touch; today when left out. */
  markDone: (id: number, on?: IsoDate) => call("mark_done", on ? { id, on } : { id }),
  addBlocked: (b: { start: IsoDate; end: IsoDate; label: string }) => call("add_blocked", b),
  deleteBlocked: (id: number) => call("delete_blocked", { id }),
  /** The day setting, with the days for Custom day saved in the same change when given. */
  setDayToggle: (dayToggle: DayToggle, customDays?: Weekday[]) =>
    call("set_day_toggle", customDays ? { dayToggle, customDays } : { dayToggle }),
  setNotifications: (on: boolean) => call("set_notifications", { on }),
  /** A step of the first-run guide is behind you (done or skipped). */
  finishGuide: (step: GuideStep) => call("finish_guide", { step }),
  /** The cards in Contacts. Asks for access only when `ask` (after the app explained why). */
  listContacts: (ask: boolean) => invoke<ContactList>("list_contacts", { ask }),
  /**
   * Opens the app's page in iOS Settings for sharing more contacts (only some shared, on iPhone), and once the user is back
   * answers the cards, with those just shared as `picked`.
   */
  shareMoreContacts: () => invoke<ContactList>("share_more_contacts"),
  /**
   * Adds the people picked in Contacts, each with their own frequency, in touch today, and links each to their card (on this device,
   * and with a link on the card the other devices find).
   */
  importContacts: (people: { name: string; contact: string; frequencyDays: number }[]) =>
    invoke<View>("import_contacts", { people }),
  notificationAccess: () => invoke<NotificationAccess>("notification_access"),
  /** Asks iOS to allow notifications (its question shows the first time), and answers what the user chose. */
  allowNotifications: () => invoke<NotificationAccess>("allow_notifications"),
  getLaunchAtLogin: () => invoke<boolean>("get_launch_at_login"),
  setLaunchAtLogin: (on: boolean) => invoke<boolean>("set_launch_at_login", { on }),
  openPreferences: () => invoke<void>("open_preferences"),
  hideDropdown: () => invoke<void>("hide_dropdown"),
  quit: () => invoke<void>("quit"),
  onViewChanged: (f: (v: View) => void): Promise<UnlistenFn> => listen<View>(VIEW_CHANGED, (e) => f(e.payload)),
  getAccount: () => invoke<Account>("get_account"),
  /**
   * Opens Sign in with Apple in the browser and resolves once it comes back (or is cancelled, which
   * isn't an error: the account just stays signed out).
   */
  signInWithApple: (attempt: string) => invoke<Account>("sign_in_with_apple", { attempt }),
  /** Stops sign-in `attempt`, even one that hasn't started waiting for the browser yet. */
  cancelSignIn: (attempt: string) => invoke<Account>("cancel_sign_in", { attempt }),
  signOut: () => invoke<Account>("sign_out"),
  /** The person a link from a contact card opened the app on, once (null when none is waiting). */
  takeShownPerson: () => invoke<number | null>("take_shown_person"),
  onShowPerson: (f: () => void): Promise<UnlistenFn> => listen(SHOW_PERSON, () => f()),
  onAccountChanged: (f: (a: Account) => void): Promise<UnlistenFn> =>
    listen<Account>(ACCOUNT_CHANGED, (e) => f(e.payload)),
};
