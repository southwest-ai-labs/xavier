import React, { useState, useEffect, useCallback } from "react";
import {
  Brain,
  Cpu,
  Database,
  Activity,
  CheckCircle2,
  AlertCircle,
  Clock,
  Sparkles,
  RefreshCw,
  Zap,
  Network,
  ArrowRight,
  Search,
} from "lucide-react";
import { motion } from "motion/react";
import { getApiUrl } from "../../api/client";

interface Brain2VramDashboardProps {
  token?: string;
}

interface HealthData {
  status: string;
  version?: string;
  database?: {
    status?: string;
    wal_size_bytes?: number;
    page_count?: number;
  };
  vector_db?: {
    status?: string;
    backend?: string;
    path?: string;
  };
  embedding?: {
    status?: string;
    model?: string;
    available?: boolean;
  };
}

interface CodeStatsData {
  status: string;
  total_files?: number;
  total_symbols?: number;
  total_imports?: number;
  languages?: Record<string, number>;
  degraded?: boolean;
  warning?: string;
}

interface QueryResultItem {
  name: string;
  type: string;
  hops: string;
  latency: string;
}

function getAuthToken(token?: string): string {
  if (token && token.trim().length > 0 && token !== "null" && token !== "undefined") {
    return token;
  }
  if (typeof window !== "undefined") {
    const stored =
      localStorage.getItem("xavier_desktop_token") ||
      localStorage.getItem("xavier_api_token");
    if (stored && stored.trim().length > 0) return stored;
  }
  return "";
}

export function Brain2VramDashboard({ token }: Brain2VramDashboardProps) {
  const [health, setHealth] = useState<HealthData | null>(null);
  const [codeStats, setCodeStats] = useState<CodeStatsData | null>(null);
  const [isLoadingStats, setIsLoadingStats] = useState(false);

  const [queryInput, setQueryInput] = useState("");
  const [queryResults, setQueryResults] = useState<QueryResultItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [queryLatency, setQueryLatency] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);

  const activeWorkspace =
    typeof localStorage !== "undefined"
      ? localStorage.getItem("xavier_active_workspace") || "default"
      : "default";

  const fetchTelemetry = useCallback(async () => {
    setIsLoadingStats(true);
    const authToken = getAuthToken(token);
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "X-Workspace-Id": activeWorkspace,
    };
    if (authToken) {
      headers["X-Xavier-Token"] = authToken;
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    try {
      const [healthRes, statsRes] = await Promise.allSettled([
        fetch(getApiUrl("/health"), { headers }),
        fetch(getApiUrl("/code/stats"), { headers }),
      ]);

      if (healthRes.status === "fulfilled" && healthRes.value.ok) {
        const hData = await healthRes.value.json();
        setHealth(hData);
      } else {
        setHealth(null);
      }

      if (statsRes.status === "fulfilled" && statsRes.value.ok) {
        const sData = await statsRes.value.json();
        setCodeStats(sData);
      } else {
        setCodeStats(null);
      }
    } catch {
      setHealth(null);
      setCodeStats(null);
    } finally {
      setIsLoadingStats(false);
    }
  }, [token, activeWorkspace]);

  useEffect(() => {
    fetchTelemetry();
  }, [fetchTelemetry]);

  const handleRunQuery = async () => {
    const trimmed = queryInput.trim();
    if (!trimmed) return;

    setIsSearching(true);
    setSearchError(null);
    setHasSearched(true);
    const t0 = performance.now();

    try {
      const authToken = getAuthToken(token);
      const headers: Record<string, string> = {
        "Content-Type": "application/json",
        "X-Workspace-Id": activeWorkspace,
      };
      if (authToken) {
        headers["X-Xavier-Token"] = authToken;
        headers["Authorization"] = `Bearer ${authToken}`;
      }

      const res = await fetch(getApiUrl("/code/find"), {
        method: "POST",
        headers,
        body: JSON.stringify({ query: trimmed, limit: 12 }),
      });

      const elapsed = (performance.now() - t0).toFixed(2);
      setQueryLatency(`${elapsed}ms`);

      if (!res.ok) {
        const errText = await res.text().catch(() => "Error en la consulta");
        setSearchError(`Error ${res.status}: ${errText}`);
        setQueryResults([]);
        return;
      }

      const data = await res.json();
      if (data.results && Array.isArray(data.results)) {
        const formatted: QueryResultItem[] = data.results.map((item: any) => ({
          name: item.name || "Símbolo sin nombre",
          type: item.kind || "Symbol",
          hops: item.file_path
            ? `${item.file_path}${item.line_number ? `:${item.line_number}` : ""}`
            : "CodeGraph",
          latency: `${elapsed}ms`,
        }));
        setQueryResults(formatted);
      } else {
        setQueryResults([]);
      }
    } catch (err: any) {
      const elapsed = (performance.now() - t0).toFixed(2);
      setQueryLatency(`${elapsed}ms`);
      setSearchError(err?.message || "Error al conectar con /code/find");
      setQueryResults([]);
    } finally {
      setIsSearching(false);
    }
  };

  // Honest WAL size calculation
  let walSize = "No disponible";
  if (health?.database?.wal_size_bytes != null) {
    const walMb = (health.database.wal_size_bytes / (1024 * 1024)).toFixed(2);
    walSize = `${walMb} MB (${health.database.status === "healthy" ? "Normal" : "Atención"})`;
  } else if (!health) {
    walSize = "Desconectado";
  }

  // Honest indexing status
  const isIndexed = Boolean(
    codeStats &&
      !codeStats.degraded &&
      typeof codeStats.total_symbols === "number" &&
      codeStats.total_symbols > 0
  );

  const totalFiles = codeStats?.total_files ?? 0;
  const totalSymbols = codeStats?.total_symbols ?? 0;
  const totalImports = codeStats?.total_imports ?? 0;

  const progressPercent = isIndexed ? 100 : 0;
  const remainingPercent = 100 - progressPercent;

  return (
    <div className="w-full max-w-6xl mx-auto py-4 sm:py-6 px-3 sm:px-4 space-y-6 text-slate-100 font-sans">
      {/* Header Banner */}
      <div className="relative overflow-hidden rounded-2xl bg-[#080808]/90 backdrop-blur-md border border-white/10 p-5 sm:p-6 shadow-2xl">
        <div className="absolute -right-20 -top-20 w-64 h-64 bg-emerald-500/10 rounded-full blur-3xl pointer-events-none" />
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center shadow-[0_0_16px_rgba(16,185,129,0.15)] text-emerald-400">
              <Brain className="w-7 h-7" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-xl sm:text-2xl font-bold tracking-tight text-white font-mono">
                  Xavier Brain 2 — Living Knowledge Engine
                </h1>
                <span className="px-2.5 py-0.5 rounded-full text-[10px] font-mono font-semibold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                  {health ? "CONECTADO" : "SIN CONEXIÓN"}
                </span>
              </div>
              <p className="text-xs text-slate-400 font-mono mt-0.5">
                Arquitectura Karpathy v2 • In-Memory CSR Graph-RAG • Dual-Plane Parity
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={() => fetchTelemetry()}
              disabled={isLoadingStats}
              className="px-3.5 py-2 bg-white/[0.04] hover:bg-white/10 border border-white/10 text-slate-300 rounded-xl text-xs font-mono font-medium transition flex items-center gap-2 shadow-sm disabled:opacity-50"
            >
              <RefreshCw className={`w-3.5 h-3.5 text-emerald-400 ${isLoadingStats ? "animate-spin" : ""}`} />
              <span>Refrescar Telemetría</span>
            </button>
            <div className="px-3 py-1.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-mono font-semibold flex items-center gap-2">
              <span className={`w-2 h-2 rounded-full ${health ? "bg-emerald-400 animate-ping" : "bg-amber-400"}`} />
              <span>
                {queryLatency
                  ? `${queryLatency} CONSULTA`
                  : health
                  ? `:8006 ${health.status.toUpperCase()}`
                  : "DESCONECTADO"}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Primary Progress Card: % Completado y % Restante */}
      <div className="rounded-2xl bg-[#080808]/80 backdrop-blur-md border border-white/10 p-5 sm:p-6 space-y-4 shadow-xl">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2">
          <div>
            <div className="flex items-center gap-2">
              <Activity className="w-4 h-4 text-emerald-400" />
              <h2 className="text-base font-semibold text-slate-200">
                Progreso del Pipeline de Indexación AST & Grafo
              </h2>
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              Extracción continua de sintaxis, análisis de dependencias cruzadas y resolución de llamadas
            </p>
          </div>
          <div className="flex items-center gap-3 font-mono">
            <span className="text-xs text-slate-400 bg-black/40 px-2.5 py-1 rounded-lg border border-white/10">
              WAL: <strong className="text-emerald-400">{walSize}</strong>
            </span>
            <span
              className={`text-xs px-2.5 py-1 rounded-lg border font-semibold ${
                isIndexed
                  ? "text-emerald-400 bg-emerald-950/40 border-emerald-500/30"
                  : "text-amber-400 bg-amber-950/40 border-amber-500/30"
              }`}
            >
              Restante: {remainingPercent}%
            </span>
          </div>
        </div>

        {/* Progress Bar with Percentages */}
        <div className="space-y-1.5">
          <div className="flex justify-between text-xs font-mono">
            <span className="text-emerald-400 font-semibold">{progressPercent}% COMPLETADO</span>
            <span className={isIndexed ? "text-emerald-400 font-semibold" : "text-amber-400 font-semibold"}>
              {isIndexed ? "INDEXACIÓN COMPLETA" : "PENDIENTE DE INDEXACIÓN (xavier code scan .)"}
            </span>
          </div>
          <div className="w-full h-3 bg-black/60 rounded-full overflow-hidden p-0.5 border border-white/10">
            <motion.div
              className={`h-full rounded-full shadow-lg relative overflow-hidden ${
                isIndexed
                  ? "bg-gradient-to-r from-emerald-500 via-teal-400 to-emerald-400 shadow-emerald-500/40"
                  : "bg-amber-600/40 shadow-amber-500/20"
              }`}
              initial={{ width: "0%" }}
              animate={{ width: `${progressPercent}%` }}
              transition={{ duration: 0.5, ease: "easeOut" }}
            >
              {isIndexed && <div className="absolute inset-0 bg-white/20 animate-pulse" />}
            </motion.div>
          </div>
        </div>

        {/* 4 Pipeline Stages */}
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3 pt-2">
          <div className={`p-3.5 rounded-xl bg-black/40 border space-y-1 ${isIndexed ? "border-emerald-500/40 bg-emerald-500/5" : "border-white/10"}`}>
            <div className="flex items-center justify-between text-xs">
              <span className={isIndexed ? "text-emerald-400 font-bold" : "text-slate-300 font-bold"}>1. Extracción AST</span>
              {isIndexed ? <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" /> : <Clock className="w-3.5 h-3.5 text-slate-500" />}
            </div>
            <div className="text-[11px] text-slate-300 font-mono">
              {isIndexed ? `${totalFiles.toLocaleString()} archivos` : "0 archivos indexados"}
            </div>
            <p className="text-[10px] text-slate-400">
              {isIndexed && codeStats?.languages
                ? Object.keys(codeStats.languages).slice(0, 4).join(", ") || "Archivos de código"
                : "Ejecuta `xavier code scan .` para indexar."}
            </p>
          </div>

          <div className={`p-3.5 rounded-xl bg-black/40 border space-y-1 ${isIndexed ? "border-emerald-500/40 bg-emerald-500/5" : "border-white/10"}`}>
            <div className="flex items-center justify-between text-xs">
              <span className={isIndexed ? "text-emerald-400 font-bold" : "text-slate-300 font-bold"}>2. Grafo de Símbolos</span>
              {isIndexed ? <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" /> : <Clock className="w-3.5 h-3.5 text-slate-500" />}
            </div>
            <div className={`text-[11px] font-mono ${isIndexed ? "text-emerald-300" : "text-slate-400"}`}>
              {isIndexed ? `${totalSymbols.toLocaleString()} símbolos` : "0 símbolos"}
            </div>
            <p className="text-[10px] text-slate-400">
              {isIndexed ? `${totalImports.toLocaleString()} imports detectados.` : "Sin símbolos en el grafo."}
            </p>
          </div>

          <div className={`p-3.5 rounded-xl bg-black/40 border space-y-1 ${health ? "border-emerald-500/40 bg-emerald-500/5" : "border-white/10"}`}>
            <div className="flex items-center justify-between text-xs">
              <span className={health ? "text-emerald-400 font-bold" : "text-slate-300 font-bold"}>3. Vector DB</span>
              {health ? <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" /> : <Clock className="w-3.5 h-3.5 text-slate-500" />}
            </div>
            <div className={`text-[11px] font-mono ${health ? "text-emerald-300" : "text-slate-400"}`}>
              {health?.vector_db?.backend ? `Backend: ${health.vector_db.backend}` : "sqlite-vec"}
            </div>
            <p className="text-[10px] text-slate-400">
              {health?.vector_db?.status === "healthy" ? "Almacenamiento vector operativo." : "Estado no verificado."}
            </p>
          </div>

          <div className={`p-3.5 rounded-xl bg-black/40 border space-y-1 ${health ? "border-emerald-500/40 bg-emerald-500/5" : "border-white/10"}`}>
            <div className="flex items-center justify-between text-xs">
              <span className={health ? "text-emerald-400 font-bold" : "text-slate-300 font-bold"}>4. Estado Daemon</span>
              {health ? <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" /> : <AlertCircle className="w-3.5 h-3.5 text-amber-500" />}
            </div>
            <div className={`text-[11px] font-mono ${health ? "text-emerald-300" : "text-slate-400"}`}>
              {health ? `v${health.version || "0.2.x"} en :8006` : "Desconectado"}
            </div>
            <p className="text-[10px] text-slate-400">
              {health ? `Modo: ${health.status}` : "Daemon no alcanzable en :8006"}
            </p>
          </div>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-4">
        <div className="p-4 rounded-xl bg-[#080808]/80 backdrop-blur-md border border-white/10 flex items-center gap-3 shadow-lg">
          <div className="w-10 h-10 rounded-lg bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400">
            <Cpu className="w-5 h-5" />
          </div>
          <div>
            <div className="text-xs text-slate-400 font-medium font-mono">REST Daemon</div>
            <div className="text-sm font-bold text-slate-100 flex items-center gap-1.5 font-mono">
              <span className={`w-2 h-2 rounded-full ${health ? "bg-emerald-400" : "bg-red-400"}`} />
              {health ? `:8006 v${health.version || "0.2.x"}` : ":8006 Desconectado"}
            </div>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-[#080808]/80 backdrop-blur-md border border-white/10 flex items-center gap-3 shadow-lg">
          <div className="w-10 h-10 rounded-lg bg-purple-500/10 border border-purple-500/20 flex items-center justify-center text-purple-400">
            <Network className="w-5 h-5" />
          </div>
          <div>
            <div className="text-xs text-slate-400 font-medium font-mono">MCP Protocol</div>
            <div className="text-sm font-bold text-slate-100 flex items-center gap-1.5 font-mono">
              <span className={`w-2 h-2 rounded-full ${health ? "bg-emerald-400" : "bg-slate-500"}`} />
              {health ? ":8100 Configurado" : ":8100 Sin verificar"}
            </div>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-[#080808]/80 backdrop-blur-md border border-white/10 flex items-center gap-3 shadow-lg">
          <div className="w-10 h-10 rounded-lg bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-400">
            <Database className="w-5 h-5" />
          </div>
          <div>
            <div className="text-xs text-slate-400 font-medium font-mono">Dual-Plane Storage</div>
            <div className="text-sm font-bold text-slate-100 font-mono">
              {health?.vector_db?.backend ? `SQLite (${health.vector_db.backend})` : "SQLite + Vault MD"}
            </div>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-[#080808]/80 backdrop-blur-md border border-white/10 flex items-center gap-3 shadow-lg">
          <div className="w-10 h-10 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400">
            <Zap className="w-5 h-5" />
          </div>
          <div>
            <div className="text-xs text-slate-400 font-medium font-mono">CodeGraph Symbols</div>
            <div className={`text-sm font-bold font-mono ${isIndexed ? "text-emerald-400" : "text-slate-400"}`}>
              {isIndexed ? `${totalSymbols.toLocaleString()} Símbolos` : "Sin indexar"}
            </div>
          </div>
        </div>
      </div>

      {/* Interactive CSR Graph Explorer */}
      <div className="rounded-2xl bg-[#080808]/80 backdrop-blur-md border border-white/10 p-5 sm:p-6 space-y-4 shadow-xl">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-emerald-400" />
            <h2 className="text-base font-semibold text-slate-200">
              Explorador de Grafo de Código (/code/find)
            </h2>
          </div>
          {queryLatency && (
            <span className="text-xs text-emerald-400 font-mono">{queryLatency} tiempo de respuesta</span>
          )}
        </div>

        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="w-4 h-4 text-slate-400 absolute left-3.5 top-3" />
            <input
              type="text"
              value={queryInput}
              onChange={(e) => setQueryInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleRunQuery();
              }}
              placeholder="Buscar símbolo o entidad en el grafo (ej. Memory, State, handler)..."
              className="w-full bg-black/60 border border-white/15 rounded-xl pl-10 pr-4 py-2.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-emerald-500/60 transition-colors"
            />
          </div>
          <button
            onClick={handleRunQuery}
            disabled={isSearching || !queryInput.trim()}
            className="px-5 py-2.5 bg-emerald-500/20 hover:bg-emerald-500/30 border border-emerald-500/40 text-emerald-300 rounded-xl text-xs font-mono font-medium transition flex items-center gap-2 shadow-[0_0_12px_rgba(16,185,129,0.2)] disabled:opacity-50"
          >
            {isSearching ? <RefreshCw className="w-3.5 h-3.5 animate-spin" /> : <ArrowRight className="w-3.5 h-3.5" />}
            <span>Consultar Grafo</span>
          </button>
        </div>

        {searchError && (
          <div className="p-3 rounded-xl bg-red-950/40 border border-red-500/30 text-red-300 text-xs font-mono">
            {searchError}
          </div>
        )}

        {/* Results List */}
        <div className="rounded-xl bg-black/40 border border-white/10 p-3.5 space-y-2">
          <div className="text-slate-400 text-[11px] font-mono flex items-center justify-between">
            <span>
              {hasSearched
                ? `// Resultados de la consulta '${queryInput}':`
                : "// Ingresa un término para consultar el grafo de código:"}
            </span>
            <span className="text-emerald-400">
              {queryResults.length} {queryResults.length === 1 ? "resultado" : "resultados"}
            </span>
          </div>

          {queryResults.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs font-mono">
              {queryResults.map((item, idx) => (
                <div
                  key={idx}
                  className="p-2.5 rounded-lg bg-white/[0.03] border border-white/10 flex items-center justify-between hover:border-emerald-500/30 transition"
                >
                  <div className="truncate pr-2">
                    <span className="text-slate-200 font-medium">{item.name}</span>
                    <span className="block text-[10px] text-slate-500 truncate">{item.hops}</span>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span className="text-[10px] px-2 py-0.5 rounded bg-white/10 text-slate-300 border border-white/5">
                      {item.type}
                    </span>
                    <span className="text-[10px] text-emerald-400">{item.latency}</span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="py-6 text-center text-xs text-slate-500 font-mono">
              {hasSearched
                ? "No se encontraron símbolos para esta consulta en el CodeGraph actual."
                : "Sin consultas activas. Escribe un símbolo arriba y pulsa 'Consultar Grafo'."}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
