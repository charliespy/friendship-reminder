/**
 * The app's commands, as the Rust side (friends-core's `App::call`) answers them. Every command returns
 * the whole view, so the UI never works out a schedule itself.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** A calendar date, "YYYY-MM-DD". */
export type IsoDate = string;

export type DayToggle = "allDays" | "weekdaysOnly" | "weekendsOnly";

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
}

export interface Blocked {
  id: number;
  start: IsoDate;
  end: IsoDate;
  label: string;
}

export interface Settings {
  dayToggle: DayToggle;
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

/** Sent to every window after any change, with the new view. */
export const VIEW_CHANGED = "view-changed";

function call(cmd: string, args: Record<string, unknown> = {}): Promise<View> {
  return invoke<View>("call", { cmd, args });
}

export const api = {
  getView: () => call("get_view"),
  addPerson: (p: { name: string; frequencyDays: number; lastConnected: IsoDate }) => call("add_person", p),
  updatePerson: (p: { id: number; name: string; frequencyDays: number; lastConnected: IsoDate }) =>
    call("update_person", p),
  deletePeople: (ids: number[]) => call("delete_people", { ids }),
  markDone: (id: number) => call("mark_done", { id }),
  addBlocked: (b: { start: IsoDate; end: IsoDate; label: string }) => call("add_blocked", b),
  deleteBlocked: (id: number) => call("delete_blocked", { id }),
  setDayToggle: (dayToggle: DayToggle) => call("set_day_toggle", { dayToggle }),
  setNotifications: (on: boolean) => call("set_notifications", { on }),
  getLaunchAtLogin: () => invoke<boolean>("get_launch_at_login"),
  setLaunchAtLogin: (on: boolean) => invoke<boolean>("set_launch_at_login", { on }),
  openPreferences: () => invoke<void>("open_preferences"),
  hideDropdown: () => invoke<void>("hide_dropdown"),
  quit: () => invoke<void>("quit"),
  onViewChanged: (f: (v: View) => void): Promise<UnlistenFn> => listen<View>(VIEW_CHANGED, (e) => f(e.payload)),
};
