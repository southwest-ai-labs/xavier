import { Sparkles, Terminal, X } from "lucide-react";
import React from "react";

interface PureRagTopSearchBarProps {
  query: string;
  onQueryChange: (q: string) => void;
  isLoading: boolean;
  totalMatches: number;
  latencyMs: number | null;
  onClear: () => void;
}

export const PureRagTopSearchBar: React.FC<PureRagTopSearchBarProps> = ({
  query,
  onQueryChange,
  isLoading,
  totalMatches,
  latencyMs,
  onClear,
}) => {
  return (
    <div className="w-full bg-slate-900/80 backdrop-blur-md border border-white/10 rounded-2xl p-3 sm:p-4 shadow-2xl flex flex-col gap-2 transition-all">
      <div className="flex items-center gap-3">
        <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 shrink-0">
          <Sparkles className="w-5 h-5 animate-pulse" />
        </div>

        <div className="relative flex-1">
          <input
            type="text"
            data-testid="purerag-search-input"
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            placeholder="Buscar en la memoria RAG en tiempo real (Zero-AI: BM25 + Vector + PageIndex)..."
            className="w-full bg-black/40 text-white placeholder-slate-400 text-sm sm:text-base px-4 py-2.5 rounded-xl border border-white/10 focus:outline-none focus:border-emerald-500/60 focus:ring-1 focus:ring-emerald-500/40 transition-all font-mono"
            autoFocus
          />
          {query && (
            <button
              onClick={onClear}
              data-testid="purerag-clear-button"
              className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white p-1 rounded-md transition-colors"
              title="Limpiar búsqueda"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-xl bg-white/[0.04] border border-white/10 text-xs font-mono text-slate-300">
          <Terminal className="w-3.5 h-3.5 text-emerald-400" />
          <span>Zero-AI</span>
        </div>
      </div>

      {/* Meta sub-bar */}
      <div className="flex items-center justify-between text-xs font-mono text-slate-400 px-1 pt-1">
        <div className="flex items-center gap-2">
          {isLoading ? (
            <span className="text-emerald-400 flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping" />
              Buscando en la memoria...
            </span>
          ) : query ? (
            <span className="text-slate-300">
              <strong className="text-white">{totalMatches}</strong> documentos coincidentes
            </span>
          ) : (
            <span>Teclea una palabra para buscar en tiempo real</span>
          )}
        </div>

        {latencyMs !== null && query && (
          <div className="text-slate-400">
            Latencia: <span className="text-emerald-400">{latencyMs}ms</span>
          </div>
        )}
      </div>
    </div>
  );
};
