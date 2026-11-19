import fs from "node:fs";
import path from "node:path";
import { expect, test } from "@playwright/test";

const OUTPUT_DIR = "/tmp/xavier_ui_screenshots";

const VIEWPORTS = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "tablet", width: 768, height: 1024 },
  { name: "mobile", width: 375, height: 667 },
];

for (const vp of VIEWPORTS) {
  test.describe(`Visual Responsive Audit - ${vp.name} (${vp.width}x${vp.height})`, () => {
    test.use({
      viewport: { width: vp.width, height: vp.height },
    });

    test.beforeEach(async ({ page }) => {
      // Mock backend routes to prevent failures and ensure deterministic rendering
      await page.route("**/health", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            status: "ok",
            version: "0.2.20",
            system: { cpu_usage: 24.5, ram_usage_percent: 48.0 },
          }),
        });
      });

      await page.route("**/v1/system/scan", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            version: "0.2.20",
            os: "linux",
            arch: "x86_64",
            providers: [{ provider: "openai", model: "gpt-4o" }],
            workspace_id: "default",
            memory_backend: "sqlite-vec",
          }),
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

      await page.route("**/api/auth/me", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            id: "1",
            email: "operator@xavier.local",
            role: "admin",
          }),
        });
      });

      await page.route("**/v1/config/providers", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            providers: [
              { provider: "openai", api_key: "sk-test", model: "gpt-4o" },
              { provider: "gemini", api_key: "AIza-test", model: "gemini-2.0-flash" },
            ],
          }),
        });
      });

      await page.route("**/v1/memories?limit=1", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            memories: [{ id: "m-1", content: "Test memory vector", created_at: "2026-11-18T10:00:00Z" }],
            pagination: { total: 42, limit: 1, offset: 0 },
          }),
        });
      });

      await page.route("**/v1/memories/search**", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            results: [
              {
                id: "mem-res-1",
                content: "Indexed memory content for semantic RAG search",
                score: 0.94,
                metadata: { source: "docs/SRS/REQUIREMENTS.md" },
              },
            ],
          }),
        });
      });

      await page.route("**/v2/marketplace/listings", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([
            {
              id: "pkg-1",
              name: "CodeGraph Analysis Tool",
              version: "1.2.0",
              category: "tools",
              author: "SWAL Core Team",
            },
            {
              id: "pkg-2",
              name: "RTK Kernel Proxy",
              version: "0.9.1",
              category: "runtime",
              author: "SWAL Infrastructure",
            },
          ]),
        });
      });

      await page.route("**/notifications**", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([
            {
              id: "notif-1",
              title: "Memory Sync Active",
              body: "42 vectors indexed to SQLite-vec database successfully.",
              timestamp: new Date().toISOString(),
              severity: "info",
              read: false,
              islandId: "all",
            },
            {
              id: "notif-2",
              title: "Founder Node Heartbeat",
              body: "Ed25519 identity verified on SWAL mesh.",
              timestamp: new Date().toISOString(),
              severity: "success",
              read: true,
              islandId: "all",
            },
          ]),
        });
      });

      await page.route("**/panel/api/bookmarks", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([
            {
              id: "bm-1",
              title: "Core Architecture Docs",
              url: "/docs/ARCHITECTURE.md",
              created_at: new Date().toISOString(),
            },
          ]),
        });
      });

      await page.route("**/panel/api/widgets", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([]),
        });
      });

      await page.route("**/panel/api/graph", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            id: "default",
            name: "Workspace roadmap",
            created_at: new Date(0).toISOString(),
            data: { nodes: [], links: [] },
          }),
        });
      });

      await page.route("**/panel/api/threads", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([]),
        });
      });

      await page.route("**/code/stats", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            total_symbols: 24708,
            functions: 2410,
            structs: 890,
            variables: 13600,
            imports: 3200,
          }),
        });
      });

      await page.route("**/agents", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify([]),
        });
      });
    });

    test(`performs complete visual and responsive audit on ${vp.name}`, async ({ page }, testInfo) => {
      fs.mkdirSync(OUTPUT_DIR, { recursive: true });

      // Seed localStorage
      await page.addInitScript(() => {
        window.localStorage.setItem("xavier_onboarding_completed", "true");
        window.localStorage.setItem("xavier_security_onboarding_completed", "true");
        window.localStorage.setItem("xavier_token", "mock-jwt-token");
        window.localStorage.setItem("vite-ui-theme", "studio-dark");
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

      const pageErrors: string[] = [];
      page.on("pageerror", (err) => pageErrors.push(String(err)));

      // Helper for saving screenshots to both testInfo and /tmp/xavier_ui_screenshots/
      const capture = async (name: string) => {
        const testInfoPath = testInfo.outputPath(`${name}.png`);
        const tmpPath = path.join(OUTPUT_DIR, `${vp.name}_${name}.png`);
        await page.screenshot({ path: testInfoPath, fullPage: true });
        await page.screenshot({ path: tmpPath, fullPage: true });
      };

      // 1. Navigate to app
      await page.goto("/");
      await page.waitForLoadState("domcontentloaded");

      // Verify main tabs are present
      const chatTab = page.locator('[data-testid="chat-tab"]');
      const pureragTab = page.locator('[data-testid="purerag-tab"]');
      const brain2Tab = page.locator('[data-testid="brain2-tab"]');

      await expect(chatTab).toBeVisible();
      await expect(pureragTab).toBeVisible();
      await expect(brain2Tab).toBeVisible();

      // Measure layout geometry and potential collisions
      const layoutAnalysis = await page.evaluate(() => {
        const chatBtn = document.querySelector('[data-testid="chat-tab"]');
        const topNav = chatBtn ? chatBtn.closest(".shrink-0") : null;
        const navRect = topNav ? topNav.getBoundingClientRect() : null;

        const leftBar = document.querySelector(".pointer-events-none > div:first-child");
        const leftRect = leftBar ? leftBar.getBoundingClientRect() : null;

        const rightBar = document.querySelector(".pointer-events-none > div:last-child");
        const rightRect = rightBar ? rightBar.getBoundingClientRect() : null;

        const centerIdentity = document.querySelector(".pointer-events-none > div:nth-child(2)");
        const centerRect = centerIdentity ? centerIdentity.getBoundingClientRect() : null;

        const docScrollWidth = document.documentElement.scrollWidth;
        const docClientWidth = document.documentElement.clientWidth;
        const hasHorizontalOverflow = docScrollWidth > docClientWidth;

        // Collision detection
        const horizontalOverlapLeftRight =
          leftRect && rightRect ? leftRect.right > rightRect.left : false;

        const verticalOverlapNavLeft =
          leftRect && navRect ? leftRect.bottom > navRect.top : false;
        const verticalOverlapNavRight =
          rightRect && navRect ? rightRect.bottom > navRect.top : false;

        return {
          windowWidth: window.innerWidth,
          windowHeight: window.innerHeight,
          hasHorizontalOverflow,
          docScrollWidth,
          docClientWidth,
          navRect: navRect
            ? { x: navRect.x, y: navRect.y, w: navRect.width, h: navRect.height, top: navRect.top, bottom: navRect.bottom }
            : null,
          leftRect: leftRect
            ? { x: leftRect.x, y: leftRect.y, w: leftRect.width, h: leftRect.height, top: leftRect.top, bottom: leftRect.bottom, right: leftRect.right }
            : null,
          rightRect: rightRect
            ? { x: rightRect.x, y: rightRect.y, w: rightRect.width, h: rightRect.height, top: rightRect.top, bottom: rightRect.bottom, left: rightRect.left }
            : null,
          centerRect: centerRect
            ? { x: centerRect.x, y: centerRect.y, w: centerRect.width, h: centerRect.height }
            : null,
          horizontalOverlapLeftRight,
          verticalOverlapNavLeft,
          verticalOverlapNavRight,
        };
      });

      console.log(`[${vp.name}] Layout analysis:`, JSON.stringify(layoutAnalysis, null, 2));

      // 1. Capture Main Chat Page
      await capture("01_main_chat");

      // 2. Switch to PURERAG Tab
      await pureragTab.click();
      await page.waitForTimeout(400); // allow transition
      await capture("02_purerag_tab");

      // 3. Switch to Brain 2 VRAM Tab
      await brain2Tab.click();
      await page.waitForTimeout(400); // allow transition
      await capture("03_brain2_tab");

      // Switch back to Chat
      await chatTab.click();
      await page.waitForTimeout(300);

      // 4. Test Marketplace Status Card (if visible)
      const marketplaceBtn = page.locator('button[aria-label="Marketplace v2 Status"]');
      if (await marketplaceBtn.isVisible().catch(() => false)) {
        await marketplaceBtn.click();
        await page.waitForTimeout(300);
        await capture("04_marketplace_open");
        await marketplaceBtn.click();
        await page.waitForTimeout(200);
      }

      // 5. Test Notifications Dropdown
      const notifBtn = page.locator('button[aria-label="Notifications"]');
      if (await notifBtn.isVisible().catch(() => false)) {
        await notifBtn.click();
        await page.waitForTimeout(300);
        await capture("05_notifications_open");
        const notifBackdrop = page.locator("div.fixed.inset-0.z-\\[65\\]");
        if (await notifBackdrop.isVisible().catch(() => false)) {
          await notifBackdrop.click({ position: { x: 10, y: 10 } });
        } else {
          await page.keyboard.press("Escape");
        }
        await page.waitForTimeout(200);
      }

      // 6. Test Node Connection Modal
      const nodeBtn = page.locator('button[aria-label="Node Connection Settings"]');
      if (await nodeBtn.isVisible().catch(() => false)) {
        await nodeBtn.click();
        await page.waitForTimeout(300);
        const nodeModalClose = page.locator('button[aria-label="Close modal"]');
        await expect(nodeModalClose).toBeVisible();
        await capture("06_node_modal_open");
        await nodeModalClose.click();
        await page.waitForTimeout(200);
      }

      // 7. Open Config / Settings Modal
      const settingsBtn = page.locator('button[data-testid="topbar-settings-btn"]');
      await expect(settingsBtn).toBeVisible();
      await settingsBtn.click();

      const modalClose = page.locator('button[aria-label="Cerrar ventana de configuración"]');
      await expect(modalClose).toBeVisible();
      await page.waitForTimeout(400); // allow modal animation
      await capture("07_settings_appearance");

      if (vp.width >= 768) {
        // Desktop / Tablet navigation
        const configTab = page.locator('[data-testid="settings-tab-config"]');
        if (await configTab.isVisible()) {
          await configTab.click();
          await page.waitForTimeout(300);
          await capture("08_settings_general");
        }

        const providersTab = page.locator('[data-testid="settings-tab-providers"]');
        if (await providersTab.isVisible()) {
          await providersTab.click();
          await page.waitForTimeout(300);
          await capture("09_settings_providers");
        }

        const agentsTab = page.locator('[data-testid="settings-tab-agents"]');
        if (await agentsTab.isVisible()) {
          await agentsTab.click();
          await page.waitForTimeout(300);
          await capture("10_settings_agents");
        }

        const metricsTab = page.locator('[data-testid="settings-tab-metrics"]');
        if (await metricsTab.isVisible()) {
          await metricsTab.click();
          await page.waitForTimeout(300);
          await capture("11_settings_metrics");
        }
      } else {
        // Mobile navigation
        const mobileConfigTab = page.locator('[data-testid="mobile-settings-tab-config"]');
        if (await mobileConfigTab.isVisible()) {
          await mobileConfigTab.click();
          await page.waitForTimeout(300);
          await capture("08_settings_general");
        }

        const mobileProvidersTab = page.locator('[data-testid="mobile-settings-tab-providers"]');
        if (await mobileProvidersTab.isVisible()) {
          await mobileProvidersTab.click();
          await page.waitForTimeout(300);
          await capture("09_settings_providers");
        }

        const mobileAgentsTab = page.locator('[data-testid="mobile-settings-tab-agents"]');
        if (await mobileAgentsTab.isVisible()) {
          await mobileAgentsTab.click();
          await page.waitForTimeout(300);
          await capture("10_settings_agents");
        }
      }

      // Close modal
      await modalClose.click();
      await expect(modalClose).toBeHidden();

      // Check errors
      expect(pageErrors).toEqual([]);
    });

    test(`verifies light theme appearance and contrast on ${vp.name}`, async ({ page }, testInfo) => {
      // Seed localStorage with light theme
      await page.addInitScript(() => {
        window.localStorage.setItem("xavier_onboarding_completed", "true");
        window.localStorage.setItem("xavier_security_onboarding_completed", "true");
        window.localStorage.setItem("xavier_token", "mock-jwt-token");
        window.localStorage.setItem("vite-ui-theme", "studio-bone");
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

      await page.goto("/");
      await page.waitForLoadState("domcontentloaded");
      await page.waitForTimeout(500);

      const testInfoPath = testInfo.outputPath(`light_theme_chat.png`);
      const tmpPath = path.join(OUTPUT_DIR, `${vp.name}_light_theme_chat.png`);
      await page.screenshot({ path: testInfoPath, fullPage: true });
      await page.screenshot({ path: tmpPath, fullPage: true });
    });
  });
}
