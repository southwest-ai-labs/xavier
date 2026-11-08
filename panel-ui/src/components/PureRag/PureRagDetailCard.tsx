import {
  Calendar,
  Check,
  Clock,
  Copy,
  ExternalLink,
  GitBranch,
  Layers,
  MapPin,
  Network,
  Shield,
  User,
} from "lucide-react";
import React, { useState } from "react";
import type { PureRagItem } from "./types";

interface PureRagDetailCardProps {
  item: PureRagItem | null;
  onNavigateToGraph: (nodeId: string, nodeTitle: string) => void;
  query?: string;
}

function highlightMatch(text: string, query?: string): React.ReactNode {
  if (!query || !query.trim() || !text) return text;
  const terms = query
    .trim()
    .split(/\s+/)
    .filter((t) => t.length > 1)
    .map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  if (terms.length === 0) return text;

  const regex = new RegExp(`(${terms.join("|")})`, "gi");
  const parts = text.split(regex);

  return parts.map((part, i) =>
    regex.test(part) ? (
      <mark
        key={i}
        className="bg-emerald-500/30 text-emerald-200 px-0.5 rounded font-bold"
      >
        {part}
      </mark>
    ) : (
      part
    )
  );
}

export const PureRagDetailCard: React.FC<PureRagDetailCardProps> = ({
  item,
  onNavigateToGraph,
  query = "",
}) => {
  const [copied, setCopied] = useState(false);
  const [copiedPath, setCopiedPath] = useState(false);
  const [viewMode, setViewMode] = useState<"clean" | "raw">("clean");

  if (!item) {
    return (
      <div className="w-full h-full min-h-[350px] flex flex-col items-center justify-center rounded-2xl bg-slate-900/40 border border-white/10 p-6 text-slate-500 font-mono text-xs text-center">
        <Network className="w-10 h-10 mb-2 opacity-30 text-emerald-400" />
        <p>Selecciona un resultado de la izquierda para ver su trazabilidad y detalles.</p>
      </div>
    );
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(item.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleCopyPath = () => {
    navigator.clipboard.writeText(item.path);
    setCopiedPath(true);
    setTimeout(() => setCopiedPath(false), 2000);
  };

  return (
    <div className="w-full flex flex-col gap-4 p-5 rounded-2xl bg-slate-900/80 backdrop-blur-md border border-white/10 text-xs font-sans text-slate-300 shadow-2xl overflow-y-auto max-h-[calc(100vh-250px)]">
      {/* Header */}
      <div className="flex items-start justify-between gap-3 border-b border-white/10 pb-3">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-sm font-bold text-white font-mono">
              {item.title || "Detalle del Documento"}
            </span>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
              {item.similarityPercent.toFixed(1)}% Match
            </span>
            {typeof item.matchCount === "number" && item.matchCount > 0 && (
              <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/20 text-amber-300 border border-amber-500/40">
                🔥 {item.matchCount} {item.matchCount === 1 ? "mención" : "menciones"}
              </span>
            )}
          </div>
          <span className="text-[11px] font-mono text-slate-400 truncate max-w-[280px]">
            ID: {item.id}
          </span>
        </div>

        <button
          onClick={handleCopy}
          className="p-1.5 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-slate-300 hover:text-white transition-all shrink-0"
          title="Copiar contenido"
        >
          {copied ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
        </button>
      </div>

      {/* Navegación al Grafo de Información (CTA principal) */}
      <button
        onClick={() => onNavigateToGraph(item.id, item.title || item.path)}
        className="w-full py-2.5 px-4 rounded-xl bg-gradient-to-r from-emerald-600/30 to-cyan-600/30 hover:from-emerald-600/50 hover:to-cyan-600/50 border border-emerald-500/40 hover:border-emerald-400 text-white font-medium flex items-center justify-center gap-2.5 shadow-lg transition-all group"
      >
        <Network className="w-4 h-4 text-emerald-400 group-hover:scale-110 transition-transform" />
        <span>Navegar / Ver en Grafo de Nodos</span>
        <ExternalLink className="w-3.5 h-3.5 text-slate-300 group-hover:translate-x-0.5 transition-transform" />
      </button>

      {/* Grid de Metadatos de Trazabilidad */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 bg-black/40 p-3.5 rounded-xl border border-white/5 font-mono text-[11px]">
        {/* Dónde está (Location) */}
        <div className="flex flex-col gap-1 sm:col-span-2">
          <div className="flex items-center justify-between">
            <span className="text-slate-400 flex items-center gap-1.5">
              <MapPin className="w-3.5 h-3.5 text-emerald-400" />
              <span>Ubicación / Ruta:</span>
            </span>
            <button
              onClick={handleCopyPath}
              className="text-[10px] text-slate-400 hover:text-emerald-400 flex items-center gap-1 transition-colors"
              title="Copiar ruta al portapapeles"
            >
              {copiedPath ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
              <span>{copiedPath ? "Copiada" : "Copiar ruta"}</span>
            </button>
          </div>
          <span className="text-white truncate bg-white/5 p-1.5 rounded border border-white/10 select-all font-mono">
            {item.path}
          </span>
        </div>

        {/* Cuándo se escribió (Almacenamiento) */}
        <div className="flex flex-col gap-1">
          <span className="text-slate-400 flex items-center gap-1.5">
            <Calendar className="w-3.5 h-3.5 text-blue-400" />
            <span>Almacenado:</span>
          </span>
          <span className="text-slate-200">
            {item.createdAt || "Timestamp no disponible"}
          </span>
        </div>

        {/* Última consulta (Tocado) */}
        <div className="flex flex-col gap-1">
          <span className="text-slate-400 flex items-center gap-1.5">
            <Clock className="w-3.5 h-3.5 text-cyan-400" />
            <span>Última Consulta / Tocado:</span>
          </span>
          <span className="text-emerald-300 font-semibold">
            {item.lastAccessedAt || item.createdAt || "Reciente"}
          </span>
        </div>

        {/* Quién lo hizo (Author / Identity) */}
        <div className="flex flex-col gap-1">
          <span className="text-slate-400 flex items-center gap-1.5">
            <User className="w-3.5 h-3.5 text-amber-400" />
            <span>Quién lo hizo (Autor):</span>
          </span>
          <span className="text-slate-200 truncate">
            {item.author || "Sistema SWAL"}
          </span>
        </div>

        {/* Workspace */}
        <div className="flex flex-col gap-1">
          <span className="text-slate-400 flex items-center gap-1.5">
            <Layers className="w-3.5 h-3.5 text-purple-400" />
            <span>Espacio de Trabajo:</span>
          </span>
          <span className="text-slate-200 truncate">
            {item.workspaceId || "default"}
          </span>
        </div>

        {/* Clearance / Seguridad */}
        <div className="flex flex-col gap-1">
          <span className="text-slate-400 flex items-center gap-1.5">
            <Shield className="w-3.5 h-3.5 text-rose-400" />
            <span>Clearance:</span>
          </span>
          <span className="text-slate-200 truncate">
            {item.clearance || "Internal (L2)"}
          </span>
        </div>
      </div>

      {/* Trazabilidad de Conexiones */}
      <div className="flex flex-col gap-2 bg-black/40 p-3.5 rounded-xl border border-white/5 font-mono text-[11px]">
        <span className="text-slate-400 flex items-center gap-1.5 font-bold">
          <GitBranch className="w-3.5 h-3.5 text-cyan-400" />
          <span>Trazabilidad y Conectividad:</span>
        </span>

        <div className="flex flex-col gap-1 text-slate-300">
          <div>
            <span className="text-slate-500">Nodo Padre:</span>{" "}
            {item.connections?.parentId || "Root / No asignado"}
          </div>
          <div>
            <span className="text-slate-500">Clúster Semántico:</span>{" "}
            {item.connections?.clusterId || "Cluster_Primary"}
          </div>
          {item.connections?.tags && item.connections.tags.length > 0 && (
            <div className="flex items-center gap-1.5 flex-wrap mt-1">
              <span className="text-slate-500">Etiquetas:</span>
              {item.connections.tags.map((t) => (
                <span
                  key={t}
                  className="px-1.5 py-0.5 rounded bg-white/5 border border-white/10 text-emerald-400 text-[10px]"
                >
                  #{t}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Contenido / Texto Completo */}
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <span className="text-slate-400 font-mono text-xs font-bold">
            Contenido del Texto / Fragmento Indexado:
          </span>
          <div className="flex items-center gap-1 bg-white/5 p-0.5 rounded-lg border border-white/10 text-[10px] font-mono">
            <button
              onClick={() => setViewMode("clean")}
              className={`px-2 py-0.5 rounded transition-all ${
                viewMode === "clean"
                  ? "bg-emerald-500/30 text-emerald-300 font-bold"
                  : "text-slate-400 hover:text-white"
              }`}
            >
              Vista Limpia
            </button>
            <button
              onClick={() => setViewMode("raw")}
              className={`px-2 py-0.5 rounded transition-all ${
                viewMode === "raw"
                  ? "bg-emerald-500/30 text-emerald-300 font-bold"
                  : "text-slate-400 hover:text-white"
              }`}
            >
              Raw
            </button>
          </div>
        </div>
        <div className="w-full max-h-56 overflow-y-auto p-3.5 rounded-xl bg-black/60 border border-white/10 font-mono text-xs text-slate-200 whitespace-pre-wrap leading-relaxed select-text">
          {viewMode === "clean" ? highlightMatch(item.content, query) : (item.metadata?.raw || item.content)}
        </div>
      </div>
    </div>
  );
};
