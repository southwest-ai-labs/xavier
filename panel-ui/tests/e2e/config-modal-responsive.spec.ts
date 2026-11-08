import { expect, test } from "@playwright/test";

const VIEWPORTS = [
  { name: "desktop", width: 1280, height: 800, isMobile: false },
  { name: "tablet", width: 768, height: 1024, isMobile: false },
  { name: "mobile", width: 390, height: 844, isMobile: true },
];

for (const vp of VIEWPORTS) {
  test.describe(`ConfigModal Responsive Layout - ${vp.name} (${vp.width}x${vp.height})`, () => {
    test.use({
      viewport: { width: vp.width, height: vp.height },
    });

    test(`renders correctly and handles tab switching on ${vp.name}`, async ({ page }, testInfo) => {
      // Mock endpoints required for authentication and rendering
      await page.route("**/health", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ status: "ok", system: { cpu_usage: 12, ram_usage_percent: 24 } }),
        });
      });

      await page.route("**/auth/login", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            token: "mock-jwt-token",
            user: { id: "1", email: "operator@xavier.local", role: "admin" },
          }),
        });
      });

      await page.route("**/v1/auth/login", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            token: "mock-jwt-token",
            user: { id: "1", email: "operator@xavier.local", role: "admin" },
          }),
        });
      });

      await page.route("**/auth/refresh", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            access_token: "mock-jwt-token",
            refresh_token: "mock-rt-token",
          }),
        });
      });

      // Seed authenticated state
      await page.addInitScript(() => {
        window.localStorage.setItem("xavier_onboarding_completed", "true");
        window.localStorage.setItem("xavier_token", "mock-jwt-token");
        window.sessionStorage.setItem("xavier.rt", "mock-rt-token");
        window.localStorage.setItem(
          "auth-storage",
          JSON.stringify({
            state: {
              token: "mock-jwt-token",
              isAuthenticated: true,
              user: { id: "1", email: "operator@xavier.local", role: "admin" },
            },
            version: 0,
          })
        );
      });

      // Collect unhandled errors
      const pageErrors: string[] = [];
      page.on("pageerror", (err) => pageErrors.push(String(err)));

      // Navigate to app
      await page.goto("/");
      await page.waitForLoadState("domcontentloaded");

      const settingsButton = page.locator('button[aria-label="Open Control Node"], button[title="Control Node"], button[data-testid="topbar-settings-btn"]');

      // If login is shown, log in
      const loginButton = page.locator('button:has-text("INITIALIZE SESSION")');
      if (await loginButton.isVisible().catch(() => false)) {
        await page.fill('input[type="email"]', "operator@xavier.local");
        const passwordInput = page.locator('input[type="password"]');
        if (await passwordInput.isVisible()) {
          await passwordInput.fill("password123");
        }
        await loginButton.click();
      }

      // Open Settings Modal
      await expect(settingsButton.first()).toBeVisible();
      await settingsButton.first().click();

      // Ensure modal is opened
      const closeButton = page.locator('button[aria-label="Cerrar ventana de configuración"]');
      await expect(closeButton).toBeVisible();

      const desktopSidebar = page.locator('[data-testid="desktop-settings-sidebar"]');
      const mobileNav = page.locator('[data-testid="mobile-settings-nav"]');

      if (vp.width >= 768) {
        // Desktop / Tablet layout
        await expect(desktopSidebar).toBeVisible();
        await expect(mobileNav).toBeHidden();

        // Switch to General Config tab
        const configTab = page.locator('[data-testid="settings-tab-config"]');
        await expect(configTab).toBeVisible();
        await configTab.click();

        // Switch back to Appearance tab
        const appearanceTab = page.locator('[data-testid="settings-tab-appearance"]');
        await expect(appearanceTab).toBeVisible();
        await appearanceTab.click();
      } else {
        // Smartphone layout (< 768px)
        await expect(desktopSidebar).toBeHidden();
        await expect(mobileNav).toBeVisible();

        // Mobile category navigation
        const mobileConfigTab = page.locator('[data-testid="mobile-settings-tab-config"]');
        await expect(mobileConfigTab).toBeVisible();
        await mobileConfigTab.click();

        const mobileAppearanceTab = page.locator('[data-testid="mobile-settings-tab-appearance"]');
        await expect(mobileAppearanceTab).toBeVisible();
        await mobileAppearanceTab.click();
      }

      // Take screenshot of the responsive modal
      const screenshotPath = testInfo.outputPath(`config_modal_${vp.name}.png`);
      await page.screenshot({ path: screenshotPath, fullPage: true });

      // Close modal
      await closeButton.click();
      await expect(closeButton).toBeHidden();

      expect(pageErrors).toEqual([]);
    });
  });
}
