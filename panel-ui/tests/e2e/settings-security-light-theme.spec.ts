import { test, expect } from "@playwright/test";

test.describe("Settings Security Light Theme", () => {
  test("security settings page has correct luminance in light theme", async ({ page }) => {
    // Intercept endpoints with real shapes
    await page.route("**/health", async (route) => {
      await route.fulfill({ json: { status: "ok" } });
    });

    await page.route("**/auth/refresh", async (route) => {
      await route.fulfill({
        status: 200,
        json: {
          token: "mock-jwt-token"
        },
      });
    });

    // Bypass onboarding and set authenticated session + light theme
    await page.addInitScript(() => {
      window.localStorage.setItem("xavier_onboarding_completed", "true");
      window.localStorage.setItem("xavier_security_onboarding_completed", "true");
      window.localStorage.setItem("vite-ui-theme", "light");
      window.localStorage.setItem(
        "auth-storage",
        JSON.stringify({
          state: {
            token: "mock-jwt-token",
            isAuthenticated: true,
            user: { id: "1", email: "operator@xavier.local", role: "admin" }
          },
          version: 1,
        })
      );
    });

    await page.goto("/");

    // Open Settings Modal via BrainCircuit button

    // Open Settings Modal via BrainCircuit button - retry logic
    const brainButton = page.locator('button[aria-label="Open Control Node"]');
    await page.waitForLoadState('networkidle');

    await expect(brainButton).toBeVisible();
    await brainButton.click();

    // Navigate to Security tab
    const securityTab = page.locator('button[data-testid="sidebar-btn-security"]');
    if (await securityTab.isVisible()) {
        await securityTab.click();
    } else {
        // Find by text if testid not found
        await page.locator('text="Security"').first().click();
    }


    // Wait for the page to render (the heading or token section)
    await expect(page.locator("text=API Tokens").first()).toBeVisible();

    // Verify theme attribute on html
    const htmlElement = page.locator("html");
    await expect(htmlElement).toHaveAttribute("data-theme", "studio-bone");

    // We take a screenshot
    await page.screenshot({ path: test.info().outputPath("security-light-theme.png") });

    // Assert main container luminance > 0.7 and headings dark
    // The security page has a main container that contains API Tokens.
    // Let's get the background color of the first token or main container.
    // In our component, we have a section: <div className="space-y-4">
    const tokenSection = page.locator("text=API Tokens").locator("..").locator("..");

    // Actually, just find the background of `body` or a large container and text color of heading.
    const heading = page.locator("text=API Tokens").first();

    // Evaluate luminance
    const colors = await heading.evaluate((el) => {
      function getLum(colorStr: string): number {
        if (!colorStr) return 0;
        const oklabMatch = colorStr.match(/okla?b\(([\d.]+)/);
        if (oklabMatch) return parseFloat(oklabMatch[1]);
        const oklchMatch = colorStr.match(/oklch\(([\d.]+)/);
        if (oklchMatch) return parseFloat(oklchMatch[1]);
        const rgbMatch = colorStr.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
        if (rgbMatch) {
          return (0.299 * parseInt(rgbMatch[1]) + 0.587 * parseInt(rgbMatch[2]) + 0.114 * parseInt(rgbMatch[3])) / 255;
        }
        const canvas = document.createElement("canvas");
        canvas.width = canvas.height = 1;
        const ctx = canvas.getContext("2d");
        if (ctx) {
          ctx.fillStyle = colorStr;
          ctx.fillRect(0, 0, 1, 1);
          const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
          return (0.299 * r + 0.587 * g + 0.114 * b) / 255;
        }
        return 0;
      }

      const mainContainer = document.body;
      const mainStyle = window.getComputedStyle(mainContainer);
      const bgLuminance = getLum(mainStyle.backgroundColor);

      const elStyle = window.getComputedStyle(el);
      const textLuminance = getLum(elStyle.color);

      return { bgLuminance, textLuminance };
    });


    console.log('Colors:', colors);
    expect(colors.bgLuminance).toBeGreaterThan(0.7);
    expect(colors.textLuminance).toBeLessThan(0.3); // "headings dark"

    // Assert there are no page errors
    const pageErrors: string[] = [];
    page.on("pageerror", (err) => pageErrors.push(String(err)));
    expect(pageErrors).toEqual([]);
  });
});
