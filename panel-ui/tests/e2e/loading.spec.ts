import { expect, test } from "@playwright/test";

test.describe("Loading and Error UI states", () => {
  test("app renders without crashing on initial load", async ({ page }) => {
    // Intercept health check to return a stable response
    await page.route("**/health", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "ok", system: { cpu_usage: 10, ram_usage_percent: 20 } }),
      });
    });

    await page.addInitScript(() => {
      window.localStorage.setItem("xavier_onboarding_completed", "true");
      window.localStorage.setItem("xavier_token", "mock-token");
    });

    await page.goto("/");

    // Collect runtime errors
    const pageErrors: Error[] = [];
    page.on("pageerror", (err) => pageErrors.push(err));

    // Verify the root app element mounts — either authenticated shell or login page
    await expect(page.locator("#root")).toBeVisible();
    // No unhandled JS errors
    expect(pageErrors).toHaveLength(0);
  });

  test("loading spinner has correct accessibility attributes", async ({ page }) => {
    await page.route("**/health", async (route) => {
      // Delay response to trigger loading state
      await new Promise((r) => setTimeout(r, 50));
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ status: "ok", system: { cpu_usage: 10, ram_usage_percent: 20 } }),
      });
    });

    await page.goto("/");
    await expect(page.locator("#root")).toBeVisible();

    // If a loading spinner is present, verify it has role="status"
    const spinner = page.locator('[data-testid="loading-spinner"]');
    const spinnerCount = await spinner.count();
    if (spinnerCount > 0) {
      await expect(spinner.first().locator("..")).toHaveAttribute("role", "status");
    }
  });
});
