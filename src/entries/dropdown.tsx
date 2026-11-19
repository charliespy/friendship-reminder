import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Dropdown } from "../Dropdown";
import "../styles.css";
import { installBrowserBridge } from "../dev/browserBridge";

// Only browser builds for development and the end-to-end tests set this; the app's build drops it.
if (import.meta.env.VITE_BROWSER_BRIDGE === "1") {
  installBrowserBridge();
  // Show the dropdown at its real size, as it sits under the menu bar.
  document.documentElement.classList.add("in-browser");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Dropdown />
  </StrictMode>,
);
