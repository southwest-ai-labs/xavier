import { chromium } from "playwright";

async function capture() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const token = process.env.XAVIER_TOKEN || "";
  const url = `http://127.0.0.1:5173/#/session?token=${token}`;

  console.log("Navigating to:", url);
  await page.goto(url, { waitUntil: "networkidle", timeout: 15000 });

  await page.waitForTimeout(1000);

  const outPath = process.env.SCREENSHOT_PATH || "./ui_full_screenshot.png";
  await page.screenshot({ path: outPath, fullPage: true });
  console.log("Screenshot successfully saved to:", outPath);

  await browser.close();
}

capture().catch((err) => {
  console.error("Capture failed:", err);
  process.exit(1);
});
