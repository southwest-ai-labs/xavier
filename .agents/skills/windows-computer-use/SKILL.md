---
name: windows-computer-use
description: >-
  Canonical guide and automation toolset for Antigravity AI agents to perform computer use,
  window control, automated interaction, and visual screenshot verification on Windows 11.
---

# Windows Computer Use & UI Visual Verification Skill

This skill provides AI agents with the tools and procedures necessary to interact with desktop applications, webviews, and standalone windows on Windows 11, including taking high-resolution visual screenshots, clicking elements, verifying layouts, and evaluating that 100% of UI features are functional.

---

## 1. Visual Perception Architecture (Screen Capture)

When validating graphical user interfaces (UI) on Windows:
1. **Never rely on console logs alone** when assessing visual fidelity or layout bugs.
2. Use **Playwright with the native Microsoft Edge engine** (`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`) to capture pixel-perfect viewport or full-page screenshots without extra dependencies.
3. Once captured to disk as a `.png`, call the native `read_file` tool directly on the image path to visually inspect the rendered UI state.

### Running a Visual Screen Capture
```powershell
node E:\proyectosSWAL\xavier\panel-ui\scripts\capture-ui.mjs
```

### Visual Verification with `read_file`
```json
{
  "Path": "C:/Users/belal/.gemini/antigravity/brain/<conv_id>/xavier_brain2_dashboard.png"
}
```

---

## 2. Window Management Protocol

To list, focus, or activate desktop application windows on Windows 11:
- Execute `scripts/window-manager.ps1 -Action List` to enumerate all top-level windows.
- Execute `scripts/window-manager.ps1 -Action Focus -Title "Xavier"` to bring the target window to the foreground.

---

## 3. Desktop Application Architecture (Standalone Mode)

When packaging or running desktop applications:
- **Never launch the raw backend console binary (`xavier.exe`) directly from GUI shortcuts**, as Windows will spawn a black console terminal window (`conhost.exe`).
- Always launch the application via:
  ```powershell
  msedge.exe --new-window --app="http://127.0.0.1:5173/#/session?token=<token>"
  ```
  This strips all browser chrome (tabs, URL bar, navigation buttons), rendering a clean, borderless standalone desktop window with the native app icon and full GPU acceleration.
- Ensure `localStorage.setItem("xavier_desktop_token", ...)` is initialized in `AuthProvider.tsx` so the session remains permanently authenticated across restarts.

---

## 4. Perception-Reasoning-Action Loop

1. **Perceive:** Capture the current window using `capture-ui.mjs` and read the image buffer with `read_file`.
2. **Reason:** Evaluate element positions, progress percentages, active tabs, and responsive layout.
3. **Act:** Simulate clicks or inputs on the target selectors.
4. **Verify:** Take a follow-up screenshot and confirm the UI transitioned to the expected state.
