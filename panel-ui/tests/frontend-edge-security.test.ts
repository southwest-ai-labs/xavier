import { beforeEach, describe, expect, it, vi } from "vitest";
import { onRequest } from "../functions/[[path]]";
import { useAuthStore } from "../src/auth/AuthProvider";

describe("Frontend & Edge Proxy Security Saneamiento", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe("Cloudflare Pages Edge Proxy (functions/[[path]].ts)", () => {
    it("enforces Turnstile CAPTCHA on POST /auth/magic-link (rejects missing token with 403)", async () => {
      const request = new Request("https://example.com/auth/magic-link", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: "user@example.com" }),
      });

      const response = await onRequest({
        request,
        env: { TURNSTILE_SECRET_KEY: "secret" },
        next: vi.fn(),
      });

      expect(response.status).toBe(403);
      const data = await response.json();
      expect(data.ok).toBe(false);
      expect(data.error).toContain("CAPTCHA verification failed");
    });

    it("fails closed on POST /auth/magic-link if TURNSTILE_SECRET_KEY is missing in production", async () => {
      const request = new Request("https://example.com/auth/magic-link", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email: "user@example.com",
          turnstile_token: "token_123",
        }),
      });

      const response = await onRequest({
        request,
        env: { ENVIRONMENT: "production" },
        next: vi.fn(),
      });

      expect(response.status).toBe(403);
      const data = await response.json();
      expect(data.ok).toBe(false);
    });

    it("does NOT automatically inject XAVIER_API_TOKEN server-side", async () => {
      const mockFetch = vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ status: "ok" }), { status: 200 }),
      );
      vi.stubGlobal("fetch", mockFetch);

      const request = new Request("https://example.com/panel/api/ping", {
        method: "GET",
        headers: {
          // No X-Xavier-Token header sent by client
        },
      });

      await onRequest({
        request,
        env: {
          XAVIER_API_TOKEN: "secret-master-token-should-not-leak",
          XAVIER_API_ORIGIN: "https://backend.internal",
        },
        next: vi.fn(),
      });

      expect(mockFetch).toHaveBeenCalled();
      const calledInit = mockFetch.mock.calls[0][1];
      const sentHeaders = new Headers(calledInit.headers);
      expect(sentHeaders.has("X-Xavier-Token")).toBe(false);
    });

    it("respects XAVIER_CLOUD_ENDPOINT as primary origin", async () => {
      const mockFetch = vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ status: "ok" }), { status: 200 }),
      );
      vi.stubGlobal("fetch", mockFetch);

      const request = new Request("https://example.com/panel/api/status", {
        method: "GET",
      });

      await onRequest({
        request,
        env: {
          XAVIER_CLOUD_ENDPOINT: "https://custom-cloud.swal.network",
          XAVIER_API_ORIGIN: "https://fallback.swal.network",
        },
        next: vi.fn(),
      });

      expect(mockFetch).toHaveBeenCalled();
      const calledUrl = mockFetch.mock.calls[0][0];
      expect(calledUrl).toContain("https://custom-cloud.swal.network/panel/api/status");
    });
  });

  describe("AuthProvider URL hash verification (src/auth/AuthProvider.tsx)", () => {
    it("does not grant admin role or authenticated state from unverified session tokens", async () => {
      const mockFetch = vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ error: "invalid_token" }), { status: 401 }),
      );
      vi.stubGlobal("fetch", mockFetch);

      useAuthStore.setState({
        user: null,
        token: null,
        accessToken: null,
        refreshToken: null,
        isAuthenticated: false,
      });

      const verified = await useAuthStore
        .getState()
        .verifySessionToken("unverified_attacker_token", "attacker@evil.com");

      expect(verified).toBe(false);
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(false);
      expect(state.user).toBeNull();
    });

    it("sets verified user state upon 200 OK from /auth/status", async () => {
      const mockFetch = vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({
            status: "authenticated",
            user_id: "user-123",
            email: "verified@swal.network",
            role: "operator",
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      );
      vi.stubGlobal("fetch", mockFetch);

      useAuthStore.setState({
        user: null,
        token: null,
        accessToken: null,
        refreshToken: null,
        isAuthenticated: false,
      });

      const verified = await useAuthStore
        .getState()
        .verifySessionToken("valid_signed_jwt", "verified@swal.network");

      expect(verified).toBe(true);
      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.user?.email).toBe("verified@swal.network");
      expect(state.user?.role).toBe("operator");
    });
  });
});
