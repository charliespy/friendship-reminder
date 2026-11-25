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

export interface View {
  today: IsoDate;
  people: Person[];
  blocked: Blocked[];
  settings: Settings;
  unreadableFile: string | null;
}

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
  onAccountChanged: (f: (a: Account) => void): Promise<UnlistenFn> =>
    listen<Account>(ACCOUNT_CHANGED, (e) => f(e.payload)),
};
