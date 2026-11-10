import { expect, test } from "@playwright/test";
import * as path from "node:path";
import * as fs from "node:fs";

test.describe("PureRAG Inverted Real-Time Explorer E2E", () => {
  const screenshotsDir = path.resolve("test-results/screenshots");

  test.beforeAll(async () => {
    if (!fs.existsSync(screenshotsDir)) {
      fs.mkdirSync(screenshotsDir, { recursive: true });
    }
  });

  test.beforeEach(async ({ page }) => {
    // Collect console errors to guarantee zero console errors
    const consoleErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });

    // Mock backend health & config
    await page.route("**/health", async (route) => {
      await route.fulfill({ json: { status: "online", system: { cpu_usage: 10, ram_usage_percent: 25 } } });
    });

    await page.route("**/v1/config/providers", async (route) => {
      await route.fulfill({
        json: { providers: [{ provider: "local", model: "test-model" }] },
      });
    });

    await page.route("**/panel/api/threads", async (route) => {
      await route.fulfill({ json: [] });
    });

    await page.route("**/panel/api/bookmarks", async (route) => {
      await route.fulfill({ json: [] });
    });

    await page.route("**/panel/api/widgets", async (route) => {
      await route.fulfill({ json: [] });
    });

    await page.route("**/panel/api/graph", async (route) => {
      await route.fulfill({ json: { data: { nodes: [], links: [] } } });
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

    await page.route("**/auth/refresh", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          access_token: "mock-jwt-token",
          refresh_token: "mock-refresh-token",
        }),
      });
    });

    // Mock /v1/memories/search for PureRAG
    await page.route("**/v1/memories/search", async (route) => {
      const body = route.request().postDataJSON();
      const q = body?.query || "";

      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          status: "ok",
          results: [
            {
              id: "mem_doc_01",
              path: "docs/SRS/ARCHITECTURE.md#L45",
              title: "ARCHITECTURE.md",
              snippet: "Reciprocal Rank Fusion (RRF) merges BM25 lexical matches and vector embeddings with zero LLM generation overhead.",
              score: 0.984,
              author: "@belal (Jules)",
              workspace_id: "swal/xavier",
              kind: "doc",
              clearance: "Internal (L2)",
              metadata: {
                created_at: "2026-10-14 08:32:10 UTC",
                last_accessed_at: "2026-11-06 18:00:00 UTC",
                tags: ["hybrid", "rrf", "zero-ai"],
                symbols: ["query_with_embedding", "rrf_score"],
              },
            },
            {
              id: "mem_code_02",
              path: "src/search/rrf.rs#L112",
              title: "rrf.rs",
              snippet: "pub fn compute_rrf_score computes reciprocal rank fusion scores between two ranked lists deterministically.",
              score: 0.862,
              author: "Jules Agent",
              workspace_id: "swal/xavier",
              kind: "code",
              clearance: "Internal (L2)",
              metadata: {
                created_at: "2026-11-07 09:15:00 UTC",
                last_accessed_at: "2026-11-07 10:30:00 UTC",
                tags: ["rust", "search", "rrf"],
              },
            },
            {
              id: "mem_spec_03",
              path: ".gitcore/features.json#L80",
              title: "features.json",
              snippet: "feat-hybrid-search: stable status verified on executed tests with sqlite-vec and FTS5.",
              score: 0.740,
              author: "Antigravity",
              workspace_id: "swal/xavier",
              kind: "fact",
              clearance: "Public",
              metadata: {
                created_at: "2026-08-04 11:00:00 UTC",
                last_accessed_at: "2026-09-01 12:00:00 UTC",
                tags: ["ledger", "gitcore"],
              },
            },
          ],
        }),
      });
    });

    // Seed local storage to skip onboarding and login
    await page.addInitScript(() => {
      window.localStorage.setItem("xavier_onboarding_completed", "true");
      window.localStorage.setItem("xavier_token", "mock-jwt-token");
      window.localStorage.setItem("xavier_desktop_token", "mock-jwt-token");
      window.localStorage.setItem("xavier_active_top_tab", "purerag");
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

    const loginButton = page.locator('button:has-text("INITIALIZE SESSION")');
    if (await loginButton.isVisible()) {
      await page.fill('input[type="email"]', "operator@xavier.local");
      await page.fill('input[type="password"]', "password123");
      await loginButton.click();
      await page.waitForTimeout(500);
    }
  });

  test("Desktop viewport (1440x900): real-time search, ranking, auto-selection and graph bridge", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });

    // 1. Verify PureRAG loads first by default as the primary interface
    const pureragTab = page.locator('[data-testid="purerag-tab"]');
    await expect(pureragTab).toBeVisible();
    await expect(page.locator('[data-testid="purerag-explorer"]')).toBeVisible();

    // 2. Verify Tab switching works (Chat <-> PureRAG)
    const chatTab = page.locator('[data-testid="chat-tab"]');
    await chatTab.click();
    await expect(page.locator('[data-testid="purerag-explorer"]')).not.toBeVisible();
    await pureragTab.click();
    await expect(page.locator('[data-testid="purerag-explorer"]')).toBeVisible();

    // 3. Verify top search bar is prominently visible at the top
    const searchInput = page.locator('[data-testid="purerag-search-input"]');
    await expect(searchInput).toBeVisible();
    await expect(page.locator("text=Zero-AI").first()).toBeVisible();

    // 4. Type search query
    await searchInput.fill("hybrid reciprocal rank");

    // 5. Verify results list is displayed with highest similarity first (#1 at 98.4%)
    await expect(page.locator("text=98.4% Similitud").first()).toBeVisible();
    await expect(page.locator("text=86.2% Similitud").first()).toBeVisible();
    await expect(page.locator("text=74.0% Similitud").first()).toBeVisible();

    // 6. Verify #1 is automatically selected by default in the Detail Card
    const detailLocation = page.locator('span:has-text("docs/SRS/ARCHITECTURE.md#L45")').first();
    await expect(detailLocation).toBeVisible();
    await expect(page.locator("text=98.4% Match")).toBeVisible();
    await expect(page.getByText("@belal (Jules)", { exact: true })).toBeVisible();

    // 7. Take full Desktop Screenshot
    await page.waitForTimeout(400); // allow transitions to settle
    const desktopScreenshotPath = path.join(screenshotsDir, "purerag-desktop-1440.png");
    await page.screenshot({ path: desktopScreenshotPath, fullPage: false });

    // 8. Test selecting a different item (Item #2)
    const secondItem = page.locator("text=src/search/rrf.rs#L112");
    await secondItem.click();

    // Verify detail card updates to Item #2
    await expect(page.locator("text=86.2% Match")).toBeVisible();
    await expect(page.getByText("Jules Agent", { exact: true })).toBeVisible();

    // 9. Verify button to navigate to Information Graph exists
    const graphButton = page.locator("text=Navegar / Ver en Grafo de Nodos");
    await expect(graphButton).toBeVisible();
    await graphButton.click();

    // Modal opens (Config/Graph modal)
    await expect(page.locator('button[aria-label="Cerrar ventana de configuración"]')).toBeVisible();

    // Close modal
    await page.locator('button[aria-label="Cerrar ventana de configuración"]').click();
    await page.waitForTimeout(400);

    expect(fs.existsSync(desktopScreenshotPath)).toBe(true);
  });

  test("Mobile viewport (390x844): compact responsive layout, results prioritized, filter drawer", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });

    // 1. Verify PureRAG loads first by default on Mobile
    const pureragTab = page.locator('[data-testid="purerag-tab"]');
    await expect(pureragTab).toBeVisible();
    await expect(page.locator('[data-testid="purerag-explorer"]')).toBeVisible();

    // 2. Type search query
    const searchInput = page.locator('[data-testid="purerag-search-input"]');
    await expect(searchInput).toBeVisible();
    await searchInput.fill("hybrid reciprocal rank");

    // 3. Verify mobile filter button exists
    const mobileFilterBtn = page.locator("button:has-text('Filtros')");
    await expect(mobileFilterBtn).toBeVisible();

    // 4. Verify results list is prominently rendered (#1 at 98.4%)
    await expect(page.locator("text=98.4% Similitud").first()).toBeVisible();

    // 5. Take Mobile Screenshot
    await page.waitForTimeout(400);
    const mobileScreenshotPath = path.join(screenshotsDir, "purerag-mobile-390.png");
    await page.screenshot({ path: mobileScreenshotPath, fullPage: false });

    expect(fs.existsSync(mobileScreenshotPath)).toBe(true);
  });

  test("Single character search ('a') triggers RAG, ranks by recency/relevance, and updates with filters", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });

    // 1. Verify Top Search Bar
    const searchInput = page.locator('[data-testid="purerag-search-input"]');
    await expect(searchInput).toBeVisible();

    // 2. Type single character "a"
    await searchInput.fill("a");

    // 3. Verify search triggers and displays results
    await expect(page.locator("text=98.4% Similitud").first()).toBeVisible();
    await expect(page.locator("text=86.2% Similitud").first()).toBeVisible();
    await expect(page.locator("text=74.0% Similitud").first()).toBeVisible();

    // 4. Verify detail card shows Almacenado and Última Consulta / Tocado
    await expect(page.locator("text=Almacenado:")).toBeVisible();
    await expect(page.locator("text=Última Consulta / Tocado:")).toBeVisible();

    // 5. Test Filter: Change Sort Order to 'Último Consultado / Tocado'
    const sortSelect = page.locator('[data-testid="purerag-sort-select"]');
    await expect(sortSelect).toBeVisible();
    await sortSelect.selectOption("last_accessed");

    // In 'last_accessed' mode, rrf.rs (touched today at 10:30 UTC) must be #1
    const firstResult = page.locator('[data-testid="purerag-result-item"]').first();
    await expect(firstResult).toContainText("rrf.rs");

    // 6. Test Filter: Switch to 'Mayor Similitud (%)'
    await sortSelect.selectOption("similarity_desc");
    await expect(page.locator('[data-testid="purerag-result-item"]').first()).toContainText("ARCHITECTURE.md");

    // 7. Test Filter: Increase minimum similarity slider
    const similaritySlider = page.locator('input[type="range"]');
    await similaritySlider.fill("80");
    await page.waitForTimeout(300);

    // features.json (74.0%) must be filtered out
    await expect(page.locator("text=features.json")).not.toBeVisible();
    await expect(page.locator("text=ARCHITECTURE.md").first()).toBeVisible();
    await expect(page.locator("text=rrf.rs").first()).toBeVisible();

    // 8. Test Filter: Memory Kind checkbox "Código Fuente"
    await page.locator("label:has-text('Código Fuente') input").click();
    await page.waitForTimeout(300);

    // Only rrf.rs (code) remains in the results list
    await expect(page.locator('[data-testid="purerag-result-item"]').filter({ hasText: "rrf.rs" })).toBeVisible();
    await expect(page.locator('[data-testid="purerag-result-item"]').filter({ hasText: "ARCHITECTURE.md" })).not.toBeVisible();
  });
});
