import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// macOS WKWebView: while an input or the terminal has focus, the first click arrives as
// mouseup *before* mousedown, so the browser never fires `click` and every button needs
// two clicks. When a mousedown lands right after an unmatched mouseup on the same target,
// fire the lost click ourselves (after mousedown's default focus change has run).
let mousePressed = false;
let orphanUp: { target: EventTarget | null; time: number } | null = null;
window.addEventListener(
  "mousedown",
  (e) => {
    const up = orphanUp;
    orphanUp = null;
    mousePressed = true;
    if (e.button !== 0 || !up || up.target !== e.target || e.timeStamp - up.time > 100) return;
    mousePressed = false;
    setTimeout(() => e.target?.dispatchEvent(new MouseEvent("click", e)));
  },
  true,
);
window.addEventListener(
  "mouseup",
  (e) => {
    if (!mousePressed) orphanUp = { target: e.target, time: e.timeStamp };
    mousePressed = false;
  },
  true,
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
