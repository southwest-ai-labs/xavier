import { chromium } from "playwright";

async function capture() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const token = "8ae8b432a2f42cffcdf26838d9646ab429ca6582f593af66bd42e61dab6991f7";
  const url = `http://127.0.0.1:5173/#/session?token=${token}`;

  console.log("Navigating to:", url);
  await page.goto(url, { waitUntil: "networkidle", timeout: 15000 });

  await page.waitForTimeout(1000);

  const outPath = "C:/Users/belal/.gemini/antigravity/brain/70b40135-4b0b-40ae-a388-de843e7b2d27/ui_full_screenshot.png";
  await page.screenshot({ path: outPath, fullPage: true });
  console.log("Screenshot successfully saved to:", outPath);

  await browser.close();
}

capture().catch((err) => {
  console.error("Capture failed:", err);
  process.exit(1);
});
