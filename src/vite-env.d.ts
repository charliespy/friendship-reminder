/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "1" in browser builds that talk to friends-devserver instead of Tauri. */
  readonly VITE_BROWSER_BRIDGE?: string;
}
