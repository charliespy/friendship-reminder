import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ContactChoice, ContactList, View } from "./api";

type Imported = { name: string; contact: string; frequencyDays: number }[];

const listContacts = vi.fn<(ask: boolean) => Promise<ContactList>>();
const shareMoreContacts = vi.fn<() => Promise<ContactList>>();
const importContacts = vi.fn<(people: Imported) => Promise<View>>();

vi.mock("./api", async (orig) => ({
  ...(await orig<typeof import("./api")>()),
  api: {
    listContacts: (ask: boolean) => listContacts(ask),
    shareMoreContacts: () => shareMoreContacts(),
    importContacts: (people: Imported) => importContacts(people),
  },
}));

const { ContactPicker } = await import("./ContactPicker");

const renee: ContactChoice = { id: "c1", name: "Renée Martin", added: false };
const kofi: ContactChoice = { id: "c2", name: "Kofi Mensah", added: false };
const maya: ContactChoice = { id: "c3", name: "Maya Chen", added: true };
const priya: ContactChoice = { id: "c4", name: "Priya Raman", added: false };

function allowed(contacts: ContactChoice[], more: Partial<{ limited: boolean; picked: string[] }> = {}): ContactList {
  return { status: "allowed", contacts, limited: false, picked: [], ...more };
}

const onClose = vi.fn();
const onAdded = vi.fn();

function renderPicker() {
  return render(<ContactPicker closeLabel="Cancel" onClose={onClose} onAdded={onAdded} />);
}

/** The names the list shows, in order. */
function shownNames(): string[] {
  return screen.queryAllByRole("checkbox").map((b) => b.getAttribute("aria-label") ?? "");
}

function search(text: string) {
  fireEvent.change(screen.getByLabelText("Search contacts"), { target: { value: text } });
}

beforeEach(() => {
  listContacts.mockImplementation(() => Promise.resolve(allowed([renee, kofi, maya])));
  importContacts.mockImplementation(() => Promise.resolve({} as View));
});

afterEach(() => {
  cleanup();
  listContacts.mockReset();
  shareMoreContacts.mockReset();
  importContacts.mockReset();
  onClose.mockReset();
  onAdded.mockReset();
});

describe("ContactPicker search", () => {
  it("matches the words in any order, ignoring case and accents", async () => {
    renderPicker();
    await screen.findByLabelText("Renée Martin");
    expect(listContacts).toHaveBeenCalledWith(false);

    search("renee");
    expect(shownNames()).toEqual(["Renée Martin"]);
    search("MARTIN  ren");
    expect(shownNames()).toEqual(["Renée Martin"]);
    search("Renée");
    expect(shownNames()).toEqual(["Renée Martin"]);
    search("men ko");
    expect(shownNames()).toEqual(["Kofi Mensah"]);
    search("   ");
    expect(shownNames()).toEqual(["Renée Martin", "Kofi Mensah", "Maya Chen, already added"]);
  });

  it("needs every word to match, and says when nobody does", async () => {
    renderPicker();
    await screen.findByLabelText("Renée Martin");

    search("renee mensah");
    expect(shownNames()).toEqual([]);
    expect(screen.getByText("Nobody in Contacts matches “renee mensah”.")).toBeTruthy();
  });

  it("keeps a tick while the search hides that person", async () => {
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Kofi Mensah"));
    search("renee");
    expect(screen.getByRole("button", { name: "Add 1 person" })).toBeTruthy();
    search("");
    expect((screen.getByLabelText("Kofi Mensah") as HTMLInputElement).checked).toBe(true);
  });
});

describe("ContactPicker adding", () => {
  it("sends each ticked person with the frequency picked for them", async () => {
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Renée Martin"));
    fireEvent.click(screen.getByLabelText("Kofi Mensah"));
    // Ticking starts at Monthly, as Add person does.
    expect((screen.getByLabelText("How often for Kofi Mensah") as HTMLSelectElement).value).toBe("30");
    fireEvent.change(screen.getByLabelText("How often for Renée Martin"), { target: { value: "90" } });
    fireEvent.change(screen.getByLabelText("How often for Kofi Mensah"), { target: { value: "365" } });

    fireEvent.click(screen.getByRole("button", { name: "Add 2 people" }));
    await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith(2));
    expect(importContacts).toHaveBeenCalledWith([
      { name: "Renée Martin", contact: "c1", frequencyDays: 90 },
      { name: "Kofi Mensah", contact: "c2", frequencyDays: 365 },
    ]);
  });

  it("drops an unticked person and their frequency", async () => {
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Renée Martin"));
    fireEvent.change(screen.getByLabelText("How often for Renée Martin"), { target: { value: "180" } });
    fireEvent.click(screen.getByLabelText("Kofi Mensah"));
    fireEvent.click(screen.getByLabelText("Renée Martin"));
    expect(screen.queryByLabelText("How often for Renée Martin")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Add 1 person" }));
    await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith(1));
    expect(importContacts).toHaveBeenCalledWith([{ name: "Kofi Mensah", contact: "c2", frequencyDays: 30 }]);
  });

  it("shows people already in the list ticked and disabled, and never sends them", async () => {
    renderPicker();
    const already = (await screen.findByLabelText("Maya Chen, already added")) as HTMLInputElement;
    expect(already.checked).toBe(true);
    expect(already.disabled).toBe(true);
    expect(screen.getByText("Already added")).toBeTruthy();
    // Nobody new is ticked, so there's nothing to add.
    expect((screen.getByRole("button", { name: "Add people" }) as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(screen.getByLabelText("Kofi Mensah"));
    fireEvent.click(screen.getByRole("button", { name: "Add 1 person" }));
    await vi.waitFor(() => expect(importContacts).toHaveBeenCalled());
    expect(importContacts).toHaveBeenCalledWith([{ name: "Kofi Mensah", contact: "c2", frequencyDays: 30 }]);
  });

  it.each([
    ["everyone in Contacts is in the list", allowed([maya]), "Everyone in Contacts is in your list already."],
    [
      "everyone shared is in the list",
      allowed([maya], { limited: true }),
      "Everyone you've shared from Contacts is in your list already.",
    ],
    ["Contacts is empty", allowed([]), "There's nobody in Contacts yet."],
    ["nobody is shared yet", allowed([], { limited: true }), "You haven't shared anyone from Contacts yet."],
  ])("says so when %s", async (_, list, text) => {
    listContacts.mockImplementation(() => Promise.resolve(list));
    renderPicker();
    expect(await screen.findByText(text)).toBeTruthy();
  });

  it("doesn't report adding once it was closed meanwhile", async () => {
    let done: (v: View) => void = () => {};
    importContacts.mockImplementation(() => new Promise((resolve) => (done = resolve)));
    const { unmount } = renderPicker();
    fireEvent.click(await screen.findByLabelText("Kofi Mensah"));
    fireEvent.click(screen.getByRole("button", { name: "Add 1 person" }));
    expect(importContacts).toHaveBeenCalledTimes(1);
    unmount();
    done({} as View);
    await Promise.resolve();
    await Promise.resolve();
    expect(onAdded).not.toHaveBeenCalled();
  });

  it("shows what went wrong adding, and stays open", async () => {
    importContacts.mockImplementation(() => Promise.reject({ kind: "storage", message: "Couldn't save the list." }));
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Kofi Mensah"));
    fireEvent.click(screen.getByRole("button", { name: "Add 1 person" }));
    expect((await screen.findByRole("alert")).textContent).toBe("Couldn't save the list.");
    expect(onAdded).not.toHaveBeenCalled();
    expect((screen.getByLabelText("Kofi Mensah") as HTMLInputElement).checked).toBe(true);

    // The user can try again.
    importContacts.mockImplementation(() => Promise.resolve({} as View));
    const add = screen.getByRole("button", { name: "Add 1 person" }) as HTMLButtonElement;
    expect(add.disabled).toBe(false);
    fireEvent.click(add);
    await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith(1));
    expect(importContacts).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("closes from its close button", async () => {
    renderPicker();
    await screen.findByLabelText("Renée Martin");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalled();
  });
});

describe("ContactPicker access", () => {
  it("says what went wrong when Contacts can't be read, and tries again", async () => {
    listContacts.mockImplementationOnce(() => Promise.reject({ kind: "badCall", message: "The address book is busy." }));
    renderPicker();
    expect(await screen.findByText("Couldn't open Contacts")).toBeTruthy();
    expect(screen.getByText("The address book is busy.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByLabelText("Renée Martin")).toBeTruthy();
    // Trying again never asks Apple for access.
    expect(listContacts.mock.calls).toEqual([[false], [false]]);
  });

  it("explains before Apple asks, and asks on Continue", async () => {
    listContacts.mockImplementationOnce(() => Promise.resolve({ status: "notAsked" }));
    renderPicker();
    expect(await screen.findByText("Add people from your Contacts")).toBeTruthy();
    expect(listContacts).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByLabelText("Renée Martin")).toBeTruthy();
    // Only Continue asks Apple; opening the picker never does.
    expect(listContacts.mock.calls).toEqual([[false], [true]]);
  });

  it("says how to turn access on when it was denied", async () => {
    listContacts.mockImplementation(() => Promise.resolve({ status: "denied" }));
    renderPicker();
    expect(await screen.findByText("Friendship Reminder can't see your contacts")).toBeTruthy();
    expect(screen.getByText(/System Settings › Privacy & Security › Contacts/)).toBeTruthy();

    listContacts.mockImplementation(() => Promise.resolve(allowed([renee])));
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByLabelText("Renée Martin")).toBeTruthy();
    expect(listContacts.mock.calls).toEqual([[false], [false]]);
  });
});

describe("ContactPicker on iPhone", () => {
  beforeEach(() => document.documentElement.classList.add("phone-app"));
  afterEach(() => document.documentElement.classList.remove("phone-app"));

  it("says what iOS asks next", async () => {
    listContacts.mockImplementation(() => Promise.resolve({ status: "notAsked" }));
    renderPicker();
    expect(await screen.findByText(/Next, iOS asks whether to share your contacts/)).toBeTruthy();
    expect(screen.queryByText(/macOS/)).toBeNull();
    // Unlike the Mac, nothing is focused for the user on iPhone.
    expect(document.activeElement).not.toBe(screen.getByRole("button", { name: "Continue" }));
  });

  it("says where to turn access on in iOS Settings", async () => {
    listContacts.mockImplementation(() => Promise.resolve({ status: "denied" }));
    renderPicker();
    expect(await screen.findByText(/Settings › Apps › Friendship Reminder/)).toBeTruthy();
    expect(screen.queryByText(/System Settings/)).toBeNull();
  });

  it("doesn't focus the search, which would open the keyboard", async () => {
    renderPicker();
    await screen.findByLabelText("Renée Martin");
    expect(document.activeElement).not.toBe(screen.getByLabelText("Search contacts"));
  });
  /** Continue pressed from the keyboard (a tap focuses nothing), answered with `list`. */
  async function continueFromKeyboard(list: ContactList) {
    listContacts.mockImplementationOnce(() => Promise.resolve({ status: "notAsked" }));
    listContacts.mockImplementationOnce(() => Promise.resolve(list));
    renderPicker();
    const go = await screen.findByRole("button", { name: "Continue" });
    go.focus();
    fireEvent.click(go);
  }

  it("after Continue from the keyboard, focuses the first person, not the search", async () => {
    await continueFromKeyboard(allowed([renee, kofi]));
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Renée Martin")));
  });

  it("after Share more from the keyboard, focuses the top of the list, not someone ticked lower down", async () => {
    listContacts.mockImplementation(() => Promise.resolve(allowed([renee, kofi], { limited: true })));
    shareMoreContacts.mockImplementation(() => Promise.resolve(allowed([renee, kofi, priya], { limited: true })));
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Kofi Mensah"));
    const more = screen.getByRole("button", { name: "Share more contacts…" });
    more.focus();
    fireEvent.click(more);
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Renée Martin")));
  });

  it("after Continue from the keyboard, focuses how often for the people chosen in Apple's sheet", async () => {
    await continueFromKeyboard(allowed([renee, kofi], { limited: true, picked: ["c2"] }));
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("How often for Kofi Mensah")));
  });

  it("after Continue from the keyboard, focuses the line saying nobody is left, not Cancel", async () => {
    await continueFromKeyboard(allowed([]));
    await waitFor(() => expect(document.activeElement).toBe(screen.getByText("There's nobody in Contacts yet.")));
    cleanup();
    await continueFromKeyboard(allowed([maya]));
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByText("Everyone in Contacts is in your list already.")),
    );
  });
});

describe("ContactPicker on the Mac", () => {
  it("says what macOS asks next, and focuses Continue", async () => {
    listContacts.mockImplementation(() => Promise.resolve({ status: "notAsked" }));
    renderPicker();
    expect(await screen.findByText(/Next, macOS asks whether Friendship Reminder may use your contacts/)).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Continue" }));
  });

  it("focuses the search", async () => {
    renderPicker();
    await screen.findByLabelText("Renée Martin");
    expect(document.activeElement).toBe(screen.getByLabelText("Search contacts"));
  });
});

describe("ContactPicker with only some contacts shared", () => {
  it("goes straight to how often for the people chosen in Apple's sheet", async () => {
    listContacts.mockImplementation(() => Promise.resolve(allowed([renee, kofi], { limited: true, picked: ["c2"] })));
    renderPicker();
    expect(await screen.findByRole("heading", { name: "How often?" })).toBeTruthy();
    expect(shownNames()).toEqual(["Kofi Mensah"]);
    expect(screen.getByRole("button", { name: "Add 1 person" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Choose others…" }));
    expect(shownNames()).toEqual(["Renée Martin", "Kofi Mensah"]);
    // Whoever was chosen in Apple's sheet stays ticked, at Monthly.
    expect((screen.getByLabelText("Kofi Mensah") as HTMLInputElement).checked).toBe(true);
    expect((screen.getByLabelText("How often for Kofi Mensah") as HTMLSelectElement).value).toBe("30");
    expect(screen.getByRole("button", { name: "Add 1 person" })).toBeTruthy();
  });

  it("keeps the ticks of people still shared after Share more, drops the others, and adds those just shared", async () => {
    listContacts.mockImplementation(() => Promise.resolve(allowed([renee, kofi], { limited: true })));
    // Back from Settings: Kofi is no longer shared, Priya was just shared.
    let back: (list: ContactList) => void = () => {};
    shareMoreContacts.mockImplementation(() => new Promise((resolve) => (back = resolve)));
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Renée Martin"));
    fireEvent.change(screen.getByLabelText("How often for Renée Martin"), { target: { value: "365" } });
    fireEvent.click(screen.getByLabelText("Kofi Mensah"));

    fireEvent.click(screen.getByRole("button", { name: "Share more contacts…" }));
    // While the user is away in Settings.
    expect(screen.getByRole("status").textContent).toBe("Share more in Settings, then come back here.");
    back(allowed([renee, priya], { limited: true, picked: ["c4"] }));
    expect(await screen.findByRole("heading", { name: "How often?" })).toBeTruthy();
    expect(shareMoreContacts).toHaveBeenCalledTimes(1);
    // Everyone ticked shows, so what's shown is what Add adds.
    expect(shownNames()).toEqual(["Renée Martin", "Priya Raman"]);
    expect((screen.getByLabelText("How often for Renée Martin") as HTMLSelectElement).value).toBe("365");
    expect((screen.getByLabelText("How often for Priya Raman") as HTMLSelectElement).value).toBe("30");

    fireEvent.click(screen.getByRole("button", { name: "Add 2 people" }));
    await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith(2));
    expect(importContacts).toHaveBeenCalledWith([
      { name: "Renée Martin", contact: "c1", frequencyDays: 365 },
      { name: "Priya Raman", contact: "c4", frequencyDays: 30 },
    ]);
  });

  it("says what went wrong when Share more fails, and tries again", async () => {
    listContacts.mockImplementation(() => Promise.resolve(allowed([renee], { limited: true })));
    shareMoreContacts.mockImplementation(() => Promise.reject({ kind: "badCall", message: "Settings didn't open." }));
    renderPicker();
    fireEvent.click(await screen.findByRole("button", { name: "Share more contacts…" }));
    expect(await screen.findByText("Couldn't open Contacts")).toBeTruthy();
    expect(screen.getByText("Settings didn't open.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByLabelText("Renée Martin")).toBeTruthy();
    expect(listContacts.mock.calls).toEqual([[false], [false]]);
  });

  it("drops the tick of someone added meanwhile", async () => {
    listContacts.mockImplementation(() => Promise.resolve(allowed([renee, kofi], { limited: true })));
    shareMoreContacts.mockImplementation(() =>
      Promise.resolve(allowed([renee, { ...kofi, added: true }], { limited: true })),
    );
    renderPicker();
    fireEvent.click(await screen.findByLabelText("Renée Martin"));
    fireEvent.click(screen.getByLabelText("Kofi Mensah"));

    fireEvent.click(screen.getByRole("button", { name: "Share more contacts…" }));
    // Nobody was just shared, so the whole list comes back.
    expect(await screen.findByLabelText("Kofi Mensah, already added")).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "How often?" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Add 1 person" }));
    await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith(1));
    expect(importContacts).toHaveBeenCalledWith([{ name: "Renée Martin", contact: "c1", frequencyDays: 30 }]);
  });
});
