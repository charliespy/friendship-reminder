import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { View } from "./api";

const getView = vi.fn<() => Promise<View>>();
const setLaunchAtLogin = vi.fn<(on: boolean) => Promise<boolean>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    getView: () => getView(),
    onViewChanged: () => Promise.resolve(() => {}),
    getLaunchAtLogin: () => Promise.resolve(false),
    setLaunchAtLogin: (on: boolean) => setLaunchAtLogin(on),
    setDayToggle: vi.fn(),
    setNotifications: vi.fn(),
  },
}));

const { useView } = await import("./useView");
const { SettingsPane } = await import("./SettingsPane");

const VIEW: View = {
  today: "2026-11-18",
  people: [],
  blocked: [],
  settings: { dayToggle: "weekendsOnly", notifications: true },
  unreadableFile: null,
};

function Probe() {
  const { view } = useView();
  return <p>{view ? view.today : "loading"}</p>;
}

/** A promise and the function that resolves it. */
function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

beforeEach(() => {
  getView.mockReset();
  setLaunchAtLogin.mockReset();
});

describe("useView", () => {
  it("asks again when the window comes forward, once while a request is on its way", async () => {
    getView.mockResolvedValueOnce(VIEW);
    render(<Probe />);
    expect(await screen.findByText("2026-11-18")).toBeTruthy();
    expect(getView).toHaveBeenCalledTimes(1);

    const later = deferred<View>();
    getView.mockReturnValueOnce(later.promise);
    // Showing a window fires both events.
    fireEvent.focus(window);
    fireEvent(document, new Event("visibilitychange"));
    expect(getView).toHaveBeenCalledTimes(2);

    await act(async () => later.resolve({ ...VIEW, today: "2026-11-19" }));
    expect(await screen.findByText("2026-11-19")).toBeTruthy();

    // Once the answer is in, the next show asks again.
    getView.mockResolvedValueOnce(VIEW);
    fireEvent.focus(window);
    expect(getView).toHaveBeenCalledTimes(3);
  });
});

describe("SettingsPane", () => {
  it("keeps Open at login disabled while a change is on its way", async () => {
    render(<SettingsPane view={VIEW} />);
    const box = screen.getByRole("checkbox", { name: /Open at login/ }) as HTMLInputElement;
    await vi.waitFor(() => expect(box.disabled).toBe(false));

    const answer = deferred<boolean>();
    setLaunchAtLogin.mockReturnValueOnce(answer.promise);
    fireEvent.click(box);
    expect(setLaunchAtLogin).toHaveBeenCalledWith(true);
    expect(box.disabled).toBe(true);

    await act(async () => answer.resolve(true));
    expect(box.disabled).toBe(false);
    expect(box.checked).toBe(true);
  });
});
