import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Preferences } from "../Preferences";
import "../styles.css";
import { installBrowserBridge } from "../dev/browserBridge";

// Only browser builds for development and the end-to-end tests set this; the app's build drops it.
if (import.meta.env.VITE_BROWSER_BRIDGE === "1") installBrowserBridge();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Preferences />
  </StrictMode>,
);
