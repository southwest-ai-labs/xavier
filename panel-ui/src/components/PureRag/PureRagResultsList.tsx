import {
  FileCode,
  FileText,
  Image as ImageIcon,
  Sparkles,
  Video as VideoIcon,
} from "lucide-react";
import React from "react";
import type { ModalityType, PureRagItem } from "./types";

interface PureRagResultsListProps {
  items: PureRagItem[];
  selectedItem: PureRagItem | null;
  onSelectItem: (item: PureRagItem) => void;
  query: string;
}

function getBadgeStyle(percent: number) {
  if (percent >= 90) {
    return "bg-emerald-500/20 text-emerald-300 border-emerald-500/50 shadow-emerald-500/10";
  }
  if (percent >= 75) {
    return "bg-cyan-500/20 text-cyan-300 border-cyan-500/50 shadow-cyan-500/10";
  }
  if (percent >= 50) {
    return "bg-amber-500/20 text-amber-300 border-amber-500/50";
  }
  return "bg-slate-500/20 text-slate-300 border-slate-500/40";
}

function getModalityIcon(modality: ModalityType) {
  switch (modality) {
    case "image":
      return <ImageIcon className="w-3.5 h-3.5 text-blue-400" />;
    case "video":
      return <VideoIcon className="w-3.5 h-3.5 text-rose-400" />;
    case "code":
      return <FileCode className="w-3.5 h-3.5 text-amber-400" />;
    default:
      return <FileText className="w-3.5 h-3.5 text-emerald-400" />;
  }
}

function highlightMatch(text: string, query: string): React.ReactNode {
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

export const PureRagResultsList: React.FC<PureRagResultsListProps> = ({
  items,
  selectedItem,
  onSelectItem,
  query,
}) => {
  if (items.length === 0) {
    return (
      <div className="w-full h-64 flex flex-col items-center justify-center rounded-2xl bg-slate-900/40 border border-dashed border-white/10 text-slate-400 font-mono text-sm p-6 text-center">
        <Sparkles className="w-8 h-8 text-slate-500 mb-2 opacity-50" />
        <p>No se encontraron documentos en la memoria RAG para esta consulta o filtros.</p>
        <span className="text-xs text-slate-500 mt-1">Prueba con palabras clave o ajusta la similitud mínima.</span>
      </div>
    );
  }

  return (
    <div className="w-full flex flex-col gap-2.5 overflow-y-auto pr-1 max-h-[calc(100vh-250px)]">
      {items.map((item, index) => {
        const isSelected = selectedItem?.id === item.id;
        const isFirst = index === 0;

        return (
          <div
            key={item.id}
            data-testid="purerag-result-item"
            onClick={() => onSelectItem(item)}
            className={`group relative p-3.5 rounded-xl border transition-all cursor-pointer ${
              isSelected
                ? "bg-emerald-950/30 border-emerald-500/60 shadow-lg shadow-emerald-500/5 ring-1 ring-emerald-500/40"
                : "bg-slate-900/60 border-white/10 hover:border-white/20 hover:bg-slate-900/80"
            }`}
          >
            {/* Header: Rank + Similarity Badge + Match Count + Modality */}
            <div className="flex items-center justify-between gap-2 mb-2">
              <div className="flex items-center gap-2">
                <span
                  className={`text-xs font-mono font-bold px-2 py-0.5 rounded-md ${
                    isFirst
                      ? "bg-emerald-500 text-black font-extrabold shadow-sm"
                      : "bg-white/10 text-slate-300"
                  }`}
                >
                  #{index + 1}
                </span>

                {item.metadata?.source === "codebase" ? (
                  <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 font-bold shrink-0">
                    Codebase
                  </span>
                ) : item.path?.includes("transcripts") || item.path?.includes("agent_memory") ? (
                  <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-300 border border-purple-500/40 font-bold shrink-0">
                    Sesión
                  </span>
                ) : null}

                <div className="flex items-center gap-1.5 text-xs text-white font-mono font-semibold truncate max-w-[170px] sm:max-w-[240px]">
                  {getModalityIcon(item.modality)}
                  <span className="truncate">{item.title || item.path}</span>
                </div>
              </div>

              {/* Badges: Match Count + Similarity */}
              <div className="flex items-center gap-1.5 shrink-0">
                {typeof item.matchCount === "number" && item.matchCount > 0 && (
                  <span
                    className="text-[10px] font-mono font-bold px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-300 border border-amber-500/40 shadow-sm"
                    title={`La palabra clave aparece ${item.matchCount} veces en este archivo`}
                  >
                    🔥 {item.matchCount}x
                  </span>
                )}
                <div
                  className={`text-xs font-mono font-bold px-2.5 py-0.5 rounded-full border shadow-sm ${getBadgeStyle(
                    item.similarityPercent
                  )}`}
                >
                  {item.similarityPercent.toFixed(1)}% Similitud
                </div>
              </div>
            </div>

            {/* Path / Reference (Cleaned location) */}
            <div className="text-[11px] font-mono text-slate-400 truncate mb-2 flex items-center gap-1">
              <span className="text-slate-500">📁</span>
              <span className="truncate">{item.path}</span>
            </div>

            {/* Content snippet focused on keyword */}
            <p className="text-xs text-slate-300 line-clamp-3 font-sans leading-relaxed">
              {highlightMatch(item.cleanExcerpt || item.excerpt || item.content, query)}
            </p>

            {/* Footer tags */}
            <div className="flex items-center gap-2 mt-3 pt-2 border-t border-white/5 text-[10px] font-mono text-slate-400">
              {item.kind && (
                <span className="px-1.5 py-0.5 rounded bg-white/5 border border-white/10">
                  {item.kind}
                </span>
              )}
              {item.workspaceId && (
                <span className="truncate max-w-[120px] text-slate-400">
                  ws: {item.workspaceId}
                </span>
              )}
              <span className="text-slate-400 ml-auto flex items-center gap-1.5">
                <span>{item.author ? `por ${item.author}` : ""}</span>
                <span className="text-emerald-400/90 font-medium">· Abierto/Tocado: {item.lastAccessedAt || item.createdAt || "Reciente"}</span>
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
};
