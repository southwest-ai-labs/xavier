import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { PureRagTopSearchBar } from "../src/components/PureRag/PureRagTopSearchBar";
import { PureRagResultsList } from "../src/components/PureRag/PureRagResultsList";
import { PureRagDetailCard } from "../src/components/PureRag/PureRagDetailCard";
import { PureRagFilterSidebar } from "../src/components/PureRag/PureRagFilterSidebar";
import type { PureRagItem } from "../src/components/PureRag/types";

import {
  countKeywordOccurrences,
  cleanRawTranscript,
  extractKeywordSnippet,
  formatHumanDate,
  parseAndCleanMemory,
} from "../src/components/PureRag/contentParser";

describe("PureRAG Components Suite", () => {
  it("renders PureRagTopSearchBar with Zero-AI indicator and handles input", () => {
    let currentQuery = "";
    const onQueryChange = (q: string) => {
      currentQuery = q;
    };
    const onClear = () => {
      currentQuery = "";
    };

    const { rerender } = render(
      <PureRagTopSearchBar
        query={currentQuery}
        onQueryChange={onQueryChange}
        isLoading={false}
        totalMatches={0}
        latencyMs={null}
        onClear={onClear}
      />
    );

    expect(screen.getByText("Zero-AI")).toBeDefined();
    expect(screen.getByPlaceholderText(/Buscar en la memoria RAG en tiempo real/i)).toBeDefined();

    const input = screen.getByPlaceholderText(/Buscar en la memoria RAG en tiempo real/i);
    fireEvent.change(input, { target: { value: "hybrid vector search" } });
    expect(currentQuery).toBe("hybrid vector search");

    rerender(
      <PureRagTopSearchBar
        query="hybrid vector search"
        onQueryChange={onQueryChange}
        isLoading={false}
        totalMatches={5}
        latencyMs={18}
        onClear={onClear}
      />
    );

    expect(screen.getByText("5")).toBeDefined();
    expect(screen.getByText("18ms")).toBeDefined();
  });

  it("renders PureRagResultsList ordered by similarity percentage with highest first", () => {
    const mockItems: PureRagItem[] = [
      {
        id: "mem_1",
        path: "docs/SRS/REQUIREMENTS.md#L45",
        content: "Reciprocal rank fusion matches with 98% accuracy.",
        score: 0.984,
        similarityPercent: 98.4,
        modality: "text",
        title: "REQUIREMENTS.md",
      },
      {
        id: "mem_2",
        path: "src/search/rrf.rs#L12",
        content: "BM25 keyword search implementation.",
        score: 0.821,
        similarityPercent: 82.1,
        modality: "code",
        title: "rrf.rs",
      },
    ];

    let selected: PureRagItem | null = mockItems[0];

    render(
      <PureRagResultsList
        items={mockItems}
        selectedItem={selected}
        onSelectItem={(item) => {
          selected = item;
        }}
        query="test"
      />
    );

    expect(screen.getByText("#1")).toBeDefined();
    expect(screen.getByText("98.4% Similitud")).toBeDefined();
    expect(screen.getByText("#2")).toBeDefined();
    expect(screen.getByText("82.1% Similitud")).toBeDefined();
  });

  it("renders PureRagDetailCard with location, author, timestamp, and graph navigation action", () => {
    const mockItem: PureRagItem = {
      id: "mem_target",
      path: "docs/SRS/ARCHITECTURE.md#L30",
      content: "Complete system architecture for Xavier vector memory.",
      score: 0.95,
      similarityPercent: 95.0,
      modality: "text",
      title: "ARCHITECTURE.md",
      createdAt: "2026-10-14 09:00:00 UTC",
      lastAccessedAt: "2026-11-01 10:00:00 UTC",
      author: "@belal",
      workspaceId: "swal/xavier",
      clearance: "Internal (L2)",
      connections: {
        parentId: "REQ-086",
        clusterId: "Cluster_Architecture",
        tags: ["purerag", "srs"],
      },
    };

    let navigatedNodeId = "";
    render(
      <PureRagDetailCard
        item={mockItem}
        onNavigateToGraph={(nodeId) => {
          navigatedNodeId = nodeId;
        }}
      />
    );

    expect(screen.getByText("docs/SRS/ARCHITECTURE.md#L30")).toBeDefined();
    expect(screen.getByText("@belal")).toBeDefined();
    expect(screen.getByText("2026-10-14 09:00:00 UTC")).toBeDefined();
    expect(screen.getByText("2026-11-01 10:00:00 UTC")).toBeDefined();
    expect(screen.getByText("95.0% Match")).toBeDefined();

    const graphBtn = screen.getByText(/Navegar \/ Ver en Grafo de Nodos/i);
    expect(graphBtn).toBeDefined();
    fireEvent.click(graphBtn);
    expect(navigatedNodeId).toBe("mem_target");
  });

  it("renders PureRagFilterSidebar with sort select and handles filter changes", () => {
    let currentFilters = {
      workspaceId: "",
      project: "",
      userId: "",
      kinds: [],
      minSimilarity: 0,
      timeHorizon: "all",
      modality: "all" as const,
      sortBy: "relevance_and_recency" as const,
    };

    const { rerender } = render(
      <PureRagFilterSidebar
        filters={currentFilters}
        onFilterChange={(f) => {
          currentFilters = f;
        }}
        onReset={() => {
          currentFilters = {
            workspaceId: "",
            project: "",
            userId: "",
            kinds: [],
            minSimilarity: 0,
            timeHorizon: "all",
            modality: "all",
            sortBy: "relevance_and_recency",
          };
        }}
      />
    );

    const sortSelect = screen.getByTestId("purerag-sort-select");
    expect(sortSelect).toBeDefined();

    fireEvent.change(sortSelect, { target: { value: "term_frequency" } });
    expect(currentFilters.sortBy).toBe("term_frequency");

    fireEvent.change(sortSelect, { target: { value: "last_accessed" } });
    expect(currentFilters.sortBy).toBe("last_accessed");

    const similaritySlider = screen.getByRole("slider");
    fireEvent.change(similaritySlider, { target: { value: "50" } });
    expect(currentFilters.minSimilarity).toBe(50);
  });

  it("contentParser cleans raw agent transcripts, counts term frequency, and extracts clean snippets", () => {
    const rawTranscript = `# Agent Conversation Session
- **IDE/Tool**: JSON_Agent
- **Last Updated**: 2026-07-25T22:21:13.713271248+00:00
- **Source DB**: /home/belal/.cursor/projects/xavier/agent-transcripts/session-123.jsonl

### RAW_JSON_EXTRACT
{"role":"user","message":{"content":[{"type":"text","text":"revisa si xavier esta funcionando y xavier responde"}]}}
{"role":"assistant","message":{"content":[{"type":"text","text":"Xavier esta activo en local"}]}}
`;

    const occurrences = countKeywordOccurrences(rawTranscript, "xavier");
    expect(occurrences).toBe(4);

    const { cleanDialogue } = cleanRawTranscript(rawTranscript);
    expect(cleanDialogue).toContain("👤 Usuario: revisa si xavier esta funcionando y xavier responde");
    expect(cleanDialogue).toContain("🤖 Asistente: Xavier esta activo en local");
    expect(cleanDialogue).not.toContain("RAW_JSON_EXTRACT");

    const snippet = extractKeywordSnippet(cleanDialogue, "xavier", 60);
    expect(snippet.toLowerCase()).toContain("xavier");

    const parsed = parseAndCleanMemory(
      {
        id: "mem_trans_01",
        memory: rawTranscript,
        metadata: {
          last_modified: "2026-07-25T22:21:13Z",
        },
      },
      "xavier"
    );

    expect(parsed.cleanPath).toContain("session-123.jsonl");
    expect(parsed.cleanTitle).toContain("revisa si xavier esta funcionando");
    expect(parsed.matchCount).toBe(5);
    expect(parsed.cleanContent).toContain("👤 Usuario:");
  });
});
