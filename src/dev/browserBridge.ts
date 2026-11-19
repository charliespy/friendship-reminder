/**
 * Stands in for Tauri's IPC when the UI runs in a plain browser (`npm run dev:web` and the end-to-end
 * tests): every `invoke` is sent to friends-devserver, which runs the real friends-core rules. Only
 * builds with VITE_BROWSER_BRIDGE=1 include it; the app never does.
 */

type Handler = (event: { event: string; id: number; payload: unknown }) => void;

export function installBrowserBridge(endpoint = "/__friends/invoke") {
  const w = window as unknown as Record<string, unknown>;
  if (w.__TAURI_INTERNALS__) return;

  let nextId = 1;
  const callbacks = new Map<number, Handler>();
  const listeners = new Map<string, number[]>();

  const emit = (event: string, payload: unknown) => {
    for (const id of listeners.get(event) ?? []) callbacks.get(id)?.({ event, id, payload });
  };

  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (event: string, id: number) =>
      listeners.set(
        event,
        (listeners.get(event) ?? []).filter((x) => x !== id),
      ),
  };

  w.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "browser" }, currentWebview: { label: "browser" } },
    transformCallback: (cb: Handler) => {
      const id = nextId++;
      callbacks.set(id, cb);
      return id;
    },
    unregisterCallback: (id: number) => callbacks.delete(id),
    invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
      if (cmd === "plugin:event|listen") {
        const event = String(args.event);
        const id = Number(args.handler);
        listeners.set(event, [...(listeners.get(event) ?? []), id]);
        return id;
      }
      if (cmd.startsWith("plugin:event|")) return null;
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ cmd, args }),
      });
      const body: unknown = await res.json();
      if (!res.ok) throw body;
      // The real shell tells every window about a new view; here there is only this window.
      if (cmd === "call") emit("view-changed", body);
      return body;
    },
  };
}
