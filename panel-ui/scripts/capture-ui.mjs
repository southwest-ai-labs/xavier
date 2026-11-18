import { chromium } from "@playwright/test";

async function capture() {
  console.log("Launching Microsoft Edge via Playwright...");
  const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
  const browser = await chromium.launch({
    executablePath: edgePath,
    headless: true
  });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1
  });
  const page = await context.newPage();

  const token = "8ae8b432a2f42cffcdf26838d9646ab429ca6582f593af66bd42e61dab6991f7";
  await page.addInitScript((tok) => {
    localStorage.setItem("xavier_desktop_token", tok);
    localStorage.setItem("xavier_token", tok);
    localStorage.setItem("xavier_onboarding_completed", "true");
    localStorage.setItem("xavier_security_onboarding_completed", "true");
  }, token);

  const url = `http://127.0.0.1:5173/#/session?token=${token}`;

  console.log("Navigating to Xavier UI:", url);
  await page.goto(url, { waitUntil: "domcontentloaded", timeout: 15000 });
  await page.waitForTimeout(3000);

  const outPath = "C:/Users/belal/.gemini/antigravity/brain/70b40135-4b0b-40ae-a388-de843e7b2d27/xavier_ui_live.png";
  await page.screenshot({ path: outPath, fullPage: true });
  console.log("Screenshot successfully saved to:", outPath);

  // Check if we are on Brain 2 tab or can click it
  const brainTab = await page.$("text=Brain 2 VRAM");
  if (brainTab) {
    console.log("Found Brain 2 VRAM tab, clicking...");
    await brainTab.click();
    await page.waitForTimeout(1000);
    const brainOutPath = "C:/Users/belal/.gemini/antigravity/brain/70b40135-4b0b-40ae-a388-de843e7b2d27/xavier_brain2_dashboard.png";
    await page.screenshot({ path: brainOutPath, fullPage: true });
    console.log("Brain 2 Dashboard screenshot saved to:", brainOutPath);
  }

  await browser.close();
  console.log("Capture completed.");
}

capture().catch((err) => {
  console.error("Capture failed:", err);
  process.exit(1);
});
