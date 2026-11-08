import {
  ArrowUpDown,
  Calendar,
  Clock,
  Filter,
  FolderGit2,
  Layers,
  RotateCcw,
  Sliders,
  User,
} from "lucide-react";
import React from "react";
import type { PureRagFiltersState } from "./types";

interface PureRagFilterSidebarProps {
  filters: PureRagFiltersState;
  onFilterChange: (filters: PureRagFiltersState) => void;
  onReset: () => void;
}

const AVAILABLE_KINDS = [
  { id: "doc", label: "Docs / Especificaciones" },
  { id: "code", label: "Código Fuente" },
  { id: "note", label: "Notas" },
  { id: "fact", label: "Hechos / L1" },
  { id: "context", label: "Contexto" },
];

export const PureRagFilterSidebar: React.FC<PureRagFilterSidebarProps> = ({
  filters,
  onFilterChange,
  onReset,
}) => {
  const toggleKind = (kindId: string) => {
    const exists = filters.kinds.includes(kindId);
    const newKinds = exists
      ? filters.kinds.filter((k) => k !== kindId)
      : [...filters.kinds, kindId];
    onFilterChange({ ...filters, kinds: newKinds });
  };

  return (
    <div className="w-full flex flex-col gap-5 p-4 rounded-2xl bg-[var(--surface)] border border-[var(--border-subtle)] text-xs font-sans text-[var(--foreground)] shadow-xl">
      <div className="flex items-center justify-between border-b border-[var(--border-subtle)] pb-3">
        <div className="flex items-center gap-2 text-[var(--foreground)] font-medium text-sm">
          <Filter className="w-4 h-4 text-emerald-400" />
          <span>Filtros de Búsqueda</span>
        </div>
        <button
          onClick={onReset}
          className="text-[var(--foreground)]/60 hover:text-[var(--foreground)] flex items-center gap-1 transition-colors font-mono text-[11px]"
          title="Restablecer filtros"
        >
          <RotateCcw className="w-3.5 h-3.5" />
          <span>Reset</span>
        </button>
      </div>

      {/* Criterio de Ordenación / Ranking */}
      <div className="flex flex-col gap-1.5 border-b border-[var(--border-subtle)] pb-3">
        <label className="text-[var(--foreground)]/70 font-mono flex items-center gap-1.5">
          <ArrowUpDown className="w-3.5 h-3.5 text-amber-400" />
          <span>Criterio de Ranking y Orden</span>
        </label>
        <select
          data-testid="purerag-sort-select"
          value={filters.sortBy || "relevance_and_recency"}
          onChange={(e) =>
            onFilterChange({
              ...filters,
              sortBy: e.target.value as any,
            })
          }
          className="w-full bg-black/40 px-3 py-1.5 rounded-lg border border-[var(--border-subtle)] text-[var(--foreground)] font-mono text-xs focus:outline-none focus:border-emerald-500/50 cursor-pointer"
        >
          <option value="term_frequency">
            🔥 Más Veces Nombrada (Frecuencia de término)
          </option>
          <option value="relevance_and_recency">
            ⭐ Relevancia + Reciente (Tocado)
          </option>
          <option value="similarity_desc">
            🎯 Mayor Similitud (%)
          </option>
          <option value="last_accessed">
            👁️ Último Consultado / Tocado
          </option>
          <option value="created_desc">
            📅 Más Reciente (Almacenado)
          </option>
        </select>
      </div>

      {/* Espacio de trabajo (Workspace) */}
      <div className="flex flex-col gap-1.5">
        <label className="text-[var(--foreground)]/70 font-mono flex items-center gap-1.5">
          <Layers className="w-3.5 h-3.5 text-blue-400" />
          <span>Espacio de Trabajo (Workspace)</span>
        </label>
        <input
          type="text"
          value={filters.workspaceId}
          onChange={(e) =>
            onFilterChange({ ...filters, workspaceId: e.target.value })
          }
          placeholder="ej: default, swal/xavier..."
          className="w-full bg-black/40 px-3 py-1.5 rounded-lg border border-[var(--border-subtle)] text-[var(--foreground)] placeholder-[var(--foreground)]/40 font-mono text-xs focus:outline-none focus:border-emerald-500/50"
        />
      </div>

      {/* Proyecto */}
      <div className="flex flex-col gap-1.5">
        <label className="text-[var(--foreground)]/70 font-mono flex items-center gap-1.5">
          <FolderGit2 className="w-3.5 h-3.5 text-purple-400" />
          <span>Proyecto</span>
        </label>
        <input
          type="text"
          value={filters.project}
          onChange={(e) =>
            onFilterChange({ ...filters, project: e.target.value })
          }
          placeholder="ej: xavier, shelf, core..."
          className="w-full bg-black/40 px-3 py-1.5 rounded-lg border border-[var(--border-subtle)] text-[var(--foreground)] placeholder-[var(--foreground)]/40 font-mono text-xs focus:outline-none focus:border-emerald-500/50"
        />
      </div>

      {/* Usuario Compartido */}
      <div className="flex flex-col gap-1.5">
        <label className="text-[var(--foreground)]/70 font-mono flex items-center gap-1.5">
          <User className="w-3.5 h-3.5 text-amber-400" />
          <span>Usuario Compartido / Autor</span>
        </label>
        <input
          type="text"
          value={filters.userId}
          onChange={(e) =>
            onFilterChange({ ...filters, userId: e.target.value })
          }
          placeholder="ej: @belal, jules, hermes..."
          className="w-full bg-black/40 px-3 py-1.5 rounded-lg border border-[var(--border-subtle)] text-[var(--foreground)] placeholder-[var(--foreground)]/40 font-mono text-xs focus:outline-none focus:border-emerald-500/50"
        />
      </div>

      {/* Umbral de Similitud (%) */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between text-[var(--foreground)]/70 font-mono">
          <span className="flex items-center gap-1.5">
            <Sliders className="w-3.5 h-3.5 text-emerald-400" />
            <span>Similitud Mínima</span>
          </span>
          <span className="text-[#39ff14] font-bold">{filters.minSimilarity}%</span>
        </div>
        <input
          type="range"
          min="0"
          max="95"
          step="5"
          value={filters.minSimilarity}
          onChange={(e) =>
            onFilterChange({
              ...filters,
              minSimilarity: Number.parseInt(e.target.value, 10),
            })
          }
          className="w-full accent-[#39ff14] cursor-pointer"
        />
      </div>

      {/* Modalidad de RAG (Texto vs Multimodal) */}
      <div className="flex flex-col gap-1.5">
        <label className="text-[var(--foreground)]/70 font-mono">Modalidad RAG</label>
        <div className="grid grid-cols-2 gap-1.5">
          {(["all", "text", "image", "video"] as const).map((m) => (
            <button
              key={m}
              onClick={() => onFilterChange({ ...filters, modality: m })}
              className={`px-2.5 py-1.5 rounded-lg border text-xs font-mono capitalize transition-all ${
                filters.modality === m
                  ? "bg-emerald-500/15 border-emerald-500/50 text-[#39ff14] font-bold shadow-[0_0_8px_rgba(57,255,20,0.15)]"
                  : "bg-white/[0.03] border-[var(--border-subtle)] text-[var(--foreground)]/60 hover:text-[var(--foreground)] hover:bg-white/5"
              }`}
            >
              {m === "all" ? "Todos" : m}
            </button>
          ))}
        </div>
      </div>

      {/* Tipo de Documento / Kind */}
      <div className="flex flex-col gap-2">
        <label className="text-[var(--foreground)]/70 font-mono">Tipo de Memoria</label>
        <div className="flex flex-col gap-1.5">
          {AVAILABLE_KINDS.map((k) => {
            const isChecked = filters.kinds.includes(k.id);
            return (
              <label
                key={k.id}
                className="flex items-center gap-2 cursor-pointer text-[var(--foreground)]/75 hover:text-[var(--foreground)] transition-colors"
              >
                <input
                  type="checkbox"
                  checked={isChecked}
                  onChange={() => toggleKind(k.id)}
                  className="rounded border-[var(--border-subtle)] bg-black/40 text-emerald-500 focus:ring-0"
                />
                <span>{k.label}</span>
              </label>
            );
          })}
        </div>
      </div>

      {/* Rango Temporal */}
      <div className="flex flex-col gap-1.5 border-t border-[var(--border-subtle)] pt-3">
        <label className="text-[var(--foreground)]/70 font-mono flex items-center gap-1.5">
          <Clock className="w-3.5 h-3.5 text-cyan-400" />
          <span>Horizonte Temporal</span>
        </label>
        <select
          value={filters.timeHorizon}
          onChange={(e) =>
            onFilterChange({ ...filters, timeHorizon: e.target.value })
          }
          className="w-full bg-black/40 px-3 py-1.5 rounded-lg border border-[var(--border-subtle)] text-[var(--foreground)] font-mono text-xs focus:outline-none focus:border-emerald-500/50 cursor-pointer"
        >
          <option value="all">Todo el historial</option>
          <option value="24h">Últimas 24 horas</option>
          <option value="7d">Últimos 7 días</option>
          <option value="30d">Últimos 30 días</option>
        </select>
      </div>
    </div>
  );
};
