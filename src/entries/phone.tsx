import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Phone } from "../Phone";
import "../styles.css";
import { installBrowserBridge } from "../dev/browserBridge";

// The phone app's sizes and colours (styles.css).
document.documentElement.classList.add("phone-app");

// Only browser builds for development and the end-to-end tests set this; the app's build drops it.
// A wide browser window shows the app at an iPhone's size; a phone-sized one fills it.
if (import.meta.env.VITE_BROWSER_BRIDGE === "1") {
  installBrowserBridge();
  if (window.innerWidth > 600) document.documentElement.classList.add("in-browser-phone");
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Phone />
  </StrictMode>,
);
