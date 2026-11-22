import path from "node:path";
import { chromium } from "@playwright/test";

async function capture() {
  console.log("Launching browser via Playwright...");
  const edgePath =
    process.env.EDGE_PATH ||
    (process.platform === "win32"
      ? "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"
      : undefined);

  const launchOptions = {
    headless: true,
  };
  if (edgePath) {
    launchOptions.executablePath = edgePath;
  }

  const browser = await chromium.launch(launchOptions);
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();

  const token = process.env.XAVIER_CAPTURE_TOKEN || "";
  if (token) {
    await page.addInitScript((tok) => {
      localStorage.setItem("xavier_desktop_token", tok);
      localStorage.setItem("xavier_token", tok);
      localStorage.setItem("xavier_onboarding_completed", "true");
      localStorage.setItem("xavier_security_onboarding_completed", "true");
    }, token);
  }

  const url = token
    ? `http://127.0.0.1:5173/#/session?token=${token}`
    : "http://127.0.0.1:5173/";

  console.log("Navigating to Xavier UI...");
  await page.goto(url, { waitUntil: "domcontentloaded", timeout: 15000 });
  await page.waitForTimeout(3000);

  const outDir = process.env.CAPTURE_OUT_DIR || process.cwd();
  const outPath = path.join(outDir, "xavier_ui_live.png");
  await page.screenshot({ path: outPath, fullPage: true });
  console.log("Screenshot successfully saved:", path.basename(outPath));

  // Check if we are on Brain 2 tab or can click it
  const brainTab = await page.$("text=Brain 2 VRAM");
  if (brainTab) {
    console.log("Found Brain 2 VRAM tab, clicking...");
    await brainTab.click();
    await page.waitForTimeout(1000);
    const brainOutPath = path.join(outDir, "xavier_brain2_dashboard.png");
    await page.screenshot({ path: brainOutPath, fullPage: true });
    console.log("Brain 2 Dashboard screenshot saved:", path.basename(brainOutPath));
  }

  await browser.close();
  console.log("Capture completed.");
}

capture().catch((err) => {
  console.error("Capture failed:", err);
  process.exit(1);
});
