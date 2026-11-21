import { render, screen, waitFor } from "@testing-library/react";
import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Brain2VramDashboard } from "../src/components/Brain2/Brain2VramDashboard";

describe("Brain2VramDashboard Component", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("renders honest offline/empty state when backend is unreachable", async () => {
    // Mock fetch failing
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("Network error"));

    render(<Brain2VramDashboard />);

    expect(screen.getByText("Xavier Brain 2 — Living Knowledge Engine")).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("SIN CONEXIÓN")).toBeInTheDocument();
    });

    expect(screen.getAllByText("Desconectado").length).toBeGreaterThan(0);
    expect(screen.getByText("0% COMPLETADO")).toBeInTheDocument();
    expect(screen.getByText("PENDIENTE DE INDEXACIÓN (xavier code scan .)")).toBeInTheDocument();
    expect(screen.getByText("0 archivos indexados")).toBeInTheDocument();
    expect(screen.getByText("0 símbolos")).toBeInTheDocument();
  });

  it("renders real telemetry when backend responds with health and code stats", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/health")) {
        return new Response(
          JSON.stringify({
            status: "healthy",
            version: "0.2.20",
            database: {
              status: "healthy",
              wal_size_bytes: 2097152, // 2.00 MB
            },
            vector_db: {
              status: "healthy",
              backend: "sqlite-vec",
            },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        );
      }
      if (url.includes("/code/stats")) {
        return new Response(
          JSON.stringify({
            status: "ok",
            total_files: 42,
            total_symbols: 350,
            total_imports: 120,
            languages: { Rust: 30, TypeScript: 12 },
            degraded: false,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        );
      }
      return new Response("Not found", { status: 404 });
    });

    render(<Brain2VramDashboard token="test-token" />);

    await waitFor(() => {
      expect(screen.getByText("CONECTADO")).toBeInTheDocument();
    });

    expect(screen.getByText("2.00 MB (Normal)")).toBeInTheDocument();
    expect(screen.getByText("100% COMPLETADO")).toBeInTheDocument();
    expect(screen.getByText("INDEXACIÓN COMPLETA")).toBeInTheDocument();
    expect(screen.getByText("42 archivos")).toBeInTheDocument();
    expect(screen.getByText("350 símbolos")).toBeInTheDocument();
    expect(screen.getByText("120 imports detectados.")).toBeInTheDocument();
    expect(screen.getByText("Backend: sqlite-vec")).toBeInTheDocument();
  });
});
