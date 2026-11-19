import {
  Activity,
  Bell,
  Bot,
  Crown,
  Database,
  Hash,
  Home,
  Key,
  MessageCircle,
  MessageSquare,
  RefreshCw,
  Send,
  Server,
  Settings,
  ShieldCheck,
  ShoppingBag,
  SlidersHorizontal,
  Users,
  Wifi,
  WifiOff,
  Zap,
} from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import React, { useEffect, useRef, useState } from "react";
import { ApiClient, getApiUrl, getRemoteUrl, setRemoteUrl } from "../api/client";
import { getApiTokenSync } from "../hooks/useApiToken";
import { useNotificationStream } from "../hooks/useNotificationStream";
import { probeMarketplaceAvailable } from "../maloca/api";
import FounderNodeStatusCard from "./FounderNodeStatusCard";
import MarketplaceStatusCard from "./MarketplaceStatusCard";
import MessagingConfigModal from "./MessagingConfigModal";
import NotificationsDropdown from "./NotificationsDropdown";
import OperationModeBadge from "./OperationModeBadge";
import WorkspaceSelector from "./WorkspaceSelector";
import NodeConnectionModal from "./modals/NodeConnectionModal";
import SyncStatusPopover from "./popovers/SyncStatusPopover";
import LoadingSpinner from "./ui/LoadingSpinner";

type MessagingPlatform =
  | "telegram"
  | "discord"
  | "slack"
  | "teams"
  | "whatsapp";

interface TopStatusBarProps {
  isModalOpen?: boolean;
  isLoading?: boolean;
  onOpenConfig?: () => void;
}

declare const __APP_VERSION__: string;

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function getToken(): string {
  return getApiTokenSync();
}

/**
 * ⚡ Bolt Performance Optimization
 * Wrapped TopStatusBar in React.memo() to avoid unnecessary tree re-renders.
 */
export default React.memo(function TopStatusBar({
  isModalOpen = false,
  isLoading: propIsLoading,
  onOpenConfig,
}: TopStatusBarProps) {
  const [time, setTime] = useState(new Date());
  const [memoryCount, setMemoryCount] = useState(0);
  const { unreadCount: streamUnreadCount } = useNotificationStream();
  const [internalLoading, setInternalLoading] = useState(true);
  const isLoading = propIsLoading !== undefined ? propIsLoading : internalLoading;
  const [metrics, setMetrics] = useState({
    cpu_percent: 0,
    ram_used_gb: 0,
    ram_total_gb: 0,
  });
  const [config, setConfig] = useState({
    has_openai: false,
    has_gemini: false,
    has_telegram: false,
  });
  const [showConfig, setShowConfig] = useState(false);
  const [showFounderCard, setShowFounderCard] = useState(false);
  const [showMarketplaceCard, setShowMarketplaceCard] = useState(false);
  const [showMessaging, setShowMessaging] = useState(false);
  const [messagingTab, setMessagingTab] = useState<MessagingPlatform>("telegram");
  const [showNotifications, setShowNotifications] = useState(false);
  const [showMobileMenu, setShowMobileMenu] = useState(false);
  const bellRef = useRef<HTMLButtonElement>(null);
  const [nodeStatus, setNodeStatus] = useState<"connected" | "disconnected" | "retrying">("connected");
  const [retryCount, setRetryCount] = useState(0);
  const [remoteUrl, setRemoteUrlState] = useState<string>(getRemoteUrl());
  const [showNodeModal, setShowNodeModal] = useState(false);
  const [liveVersion, setLiveVersion] = useState<string | null>(null);

  // Manual Memory Sync state
  const [showSyncPopover, setShowSyncPopover] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);
  const [syncLatencyMs, setSyncLatencyMs] = useState<number | null>(null);
  const [lastSyncTime, setLastSyncTime] = useState<Date | null>(null);

  const [modules, setModules] = useState({
    time: true,
    founder: true,
    marketplace: false,
    channels: true,
    resources: true,
    security: true,
    sync: true,
    ai: true,
    notifications: true,
  });

  const toggleModule = (key: keyof typeof modules) => {
    setModules((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  useEffect(() => {
    const fetchConfig = async () => {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const res = await invoke<any>("get_current_config_state");
          if (res) setConfig(res);
        } catch (err) {
          console.debug("Error fetching config state via Tauri:", err);
        }
      } else {
        try {
          const token = getToken();
          const res = await fetch(getApiUrl("/v1/config/providers"), {
            headers: { "X-Xavier-Token": token },
          });
          if (res.ok) {
            const data = await res.json();
            const providers = data.providers || [];
            const has_openai = providers.some(
              (p: any) => p.provider === "openai" && (p.api_key || p.model),
            );
            const has_gemini = providers.some(
              (p: any) => p.provider === "gemini" && (p.api_key || p.model),
            );
            setConfig((prev) => ({ ...prev, has_openai, has_gemini }));
          }
        } catch (err) {
          console.debug("Error fetching config fallback:", err);
        }
      }
    };

    fetchConfig();

    const fetchMetrics = async () => {
      try {
        const res = await fetch(getApiUrl("/health"));
        if (res.ok) {
          setNodeStatus("connected");
          setRetryCount(0);
          const data = await res.json();
          if (data.version) {
            setLiveVersion(data.version);
          }
          if (data.system) {
            const cpu_percent = data.system.cpu_usage ?? 0;
            const ram_percent = data.system.ram_usage_percent ?? 0;
            const deviceMemory =
              (navigator as unknown as { deviceMemory?: number }).deviceMemory || 8;
            const ram_used_gb = (ram_percent / 100) * deviceMemory;
            setMetrics({ cpu_percent, ram_used_gb, ram_total_gb: deviceMemory });
          }
        } else {
          setNodeStatus("disconnected");
          setRetryCount((prev) => prev + 1);
        }
      } catch (err) {
        console.debug("Error fetching metrics from /health:", err);
        setNodeStatus("disconnected");
        setRetryCount((prev) => prev + 1);
      }

      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const met = await invoke<any>("get_realtime_metrics");
          if (met) setMetrics(met);
        } catch (err) {
          console.debug("Error fetching realtime metrics via Tauri:", err);
        }
      }

      try {
        const token = getToken();
        const res = await fetch(getApiUrl("/v1/memories?limit=1"), {
          headers: { "X-Xavier-Token": token },
        });
        if (res.ok) {
          const data = await res.json();
          if (data.pagination?.total !== undefined) {
            setMemoryCount(data.pagination.total);
          }
        }
      } catch (err) {
        console.debug("Error fetching memories count:", err);
      }

      try {
        const available = await probeMarketplaceAvailable();
        setModules((prev) =>
          prev.marketplace === available ? prev : { ...prev, marketplace: available },
        );
      } finally {
        setInternalLoading(false);
      }
    };

    fetchMetrics();
    const metricsInterval = setInterval(fetchMetrics, 3000);
    const timeInterval = setInterval(() => setTime(new Date()), 1000);

    let unlisten: (() => void) | undefined;
    let notifyInterval: ReturnType<typeof setInterval> | undefined;

    if (isTauri) {
      import("@tauri-apps/api/event")
        .then(({ listen }) => {
          listen<any>("new-notification", () => {
            fetchMetrics();
          }).then((fn) => { unlisten = fn; });
        })
        .catch((err) => console.debug("Error listening for events:", err));
    } else {
      notifyInterval = setInterval(fetchMetrics, 30000);
    }

    return () => {
      clearInterval(metricsInterval);
      clearInterval(timeInterval);
      if (notifyInterval) clearInterval(notifyInterval);
      if (unlisten) unlisten();
    };
  }, []);

  const spring = { type: "spring" as const, stiffness: 200, damping: 25 };

  const openMessaging = (platform: MessagingPlatform) => {
    setMessagingTab(platform);
    setShowMessaging(true);
  };

  const handleTriggerSync = async () => {
    if (isSyncing) return;
    setIsSyncing(true);
    const startTime = performance.now();
    try {
      const token = getToken();
      const client = new ApiClient(token);
      await client.syncPush();
      await client.syncPull();

      const elapsed = Math.round(performance.now() - startTime);
      setSyncLatencyMs(elapsed);
      setLastSyncTime(new Date());

      if (typeof window !== "undefined" && window.dispatchEvent) {
        window.dispatchEvent(
          new CustomEvent("xavier-toast", {
            detail: {
              message: `Memory sync completed successfully (${elapsed}ms)`,
              type: "success",
            },
          }),
        );
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "Sync operation failed";
      if (typeof window !== "undefined" && window.dispatchEvent) {
        window.dispatchEvent(
          new CustomEvent("xavier-error-toast", {
            detail: { message: `Sync failed: ${msg}`, type: "error" },
          }),
        );
      }
    } finally {
      setIsSyncing(false);
    }
  };

  const handleSaveRemoteUrl = (url: string | null) => {
    setRemoteUrl(url);
    const active = getRemoteUrl();
    setRemoteUrlState(active);
    setShowNodeModal(false);
  };

  return (
    <>
      <div className="absolute inset-0 z-[60] pointer-events-none overflow-hidden">
        {/* Left Group */}
        <motion.div
          layout
          transition={spring}
          className={`flex gap-1.5 pointer-events-auto ${isModalOpen ? "absolute left-2 lg:left-4 top-1/2 -translate-y-1/2 flex-col items-start z-[60]" : "absolute left-2 sm:left-3 md:left-4 top-3 sm:top-4 flex-row items-center z-[60]"}`}
        >
          {/* Time & Date Pill */}
          {modules.time && (
            <motion.div
              layout
              transition={spring}
              className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-[var(--border-subtle)] shadow-md rounded-full px-3 py-1 flex items-center gap-2 h-7 text-[var(--foreground)]/80 shrink-0"
            >
              <span className="font-mono text-[10px] hidden md:inline-block">
                {time.toLocaleDateString(undefined, { month: "numeric", day: "numeric" })}
              </span>
              <div className="w-px h-2.5 bg-[var(--border-subtle)] hidden md:block" />
              <span className="font-mono text-[10px] min-w-[50px] text-center">
                {time.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}
              </span>
            </motion.div>
          )}

          {/* SWAL Genesis Founder Node Telemetry Badge */}
          {modules.founder && (
            <div className="relative hidden sm:block">
              <motion.button
                layout
                transition={spring}
                type="button"
                onClick={() => setShowFounderCard((prev) => !prev)}
                className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-amber-500/30 shadow-md rounded-full px-2.5 py-1 flex items-center gap-1.5 h-7 shrink-0 hover:bg-amber-500/10 transition-colors cursor-pointer"
                title="SWAL Genesis Founder Node Telemetry"
                aria-label="Founder Node Telemetry HUD Card"
              >
                <Crown className="w-3 h-3 text-amber-500 dark:text-amber-400" />
                <span className="font-mono text-[9px] text-amber-600 dark:text-amber-300 font-bold tracking-wider uppercase hidden sm:inline-block">
                  Founder Node
                </span>
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
              </motion.button>

              <AnimatePresence>
                {showFounderCard && (
                  <motion.div
                    initial={{ opacity: 0, y: 10, scale: 0.95 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, y: 10, scale: 0.95 }}
                    className="absolute left-0 top-full mt-2 w-72 max-w-[calc(100vw-2rem)] z-[80]"
                  >
                    <FounderNodeStatusCard onClose={() => setShowFounderCard(false)} />
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )}

          <div className="hidden sm:block">
            <OperationModeBadge />
          </div>
          <WorkspaceSelector />

          {/* Communication Channels */}
          {modules.channels && (
            <motion.div
              layout
              transition={spring}
              className="bg-[#0a0a0a]/80 backdrop-blur-md border border-white/10 shadow-lg rounded-full px-2 py-1 flex items-center gap-1.5 h-7 shrink-0 hidden lg:flex"
            >
              <button
                type="button"
                onClick={() => openMessaging("discord")}
                title="Discord — Click to configure"
                aria-label="Configure Discord"
                className="relative group p-0.5 rounded-full hover:bg-indigo-500/10 transition-colors"
              >
                <MessageCircle className="w-3 h-3 text-indigo-400/40 group-hover:text-indigo-400 transition-colors" aria-hidden="true" />
              </button>
              <button
                type="button"
                onClick={() => openMessaging("slack")}
                title="Slack — Click to configure"
                aria-label="Configure Slack"
                className="relative group p-0.5 rounded-full hover:bg-amber-500/10 transition-colors"
              >
                <Hash className="w-3 h-3 text-amber-400/40 group-hover:text-amber-400 transition-colors" aria-hidden="true" />
              </button>
              <button
                type="button"
                onClick={() => openMessaging("teams")}
                title="MS Teams — Click to configure"
                aria-label="Configure MS Teams"
                className="relative group p-0.5 rounded-full hover:bg-purple-500/10 transition-colors"
              >
                <Users className="w-3 h-3 text-purple-400/40 group-hover:text-purple-400 transition-colors" aria-hidden="true" />
              </button>
              <button
                type="button"
                onClick={() => openMessaging("telegram")}
                title={config.has_telegram ? "Telegram (Active)" : "Telegram — Click to configure"}
                aria-label={config.has_telegram ? "Telegram (Active)" : "Configure Telegram"}
                className="relative group p-0.5 rounded-full hover:bg-blue-500/10 transition-colors"
              >
                <Send className={`w-3 h-3 transition-colors ${config.has_telegram ? "text-blue-400" : "text-blue-400/40 group-hover:text-blue-400"}`} aria-hidden="true" />
                {config.has_telegram && (
                  <div className="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 rounded-full bg-blue-400 msg-active-dot shadow-[0_0_4px_rgba(96,165,250,0.6)]" aria-hidden="true" />
                )}
              </button>
              <button
                type="button"
                onClick={() => openMessaging("whatsapp")}
                title="WhatsApp — Click to configure"
                aria-label="Configure WhatsApp"
                className="relative group p-0.5 rounded-full hover:bg-green-500/10 transition-colors"
              >
                <MessageSquare className="w-3 h-3 text-green-400/40 group-hover:text-green-400 transition-colors" aria-hidden="true" />
              </button>
            </motion.div>
          )}

          {/* System Resources Pill */}
          {modules.resources && (
            <motion.div
              layout
              transition={spring}
              className="bg-[#0a0a0a]/80 backdrop-blur-md border border-white/10 shadow-lg rounded-full px-2.5 py-1 flex items-center gap-2 h-7 shrink-0 hidden lg:flex"
            >
              {isLoading ? (
                <div className="flex items-center gap-1.5 px-1">
                  <LoadingSpinner size={12} />
                  <span className="font-mono text-[10px] text-white/50">Loading...</span>
                </div>
              ) : (
                <>
                  <div
                    className="flex items-center gap-1 text-[10px] text-white/70"
                    title={`Memory: ${metrics.ram_used_gb.toFixed(1)}GB / ${metrics.ram_total_gb.toFixed(1)}GB`}
                  >
                    <Database className="w-3 h-3 text-blue-400" />
                    <span className="font-mono">{Math.round(metrics.ram_used_gb)}G</span>
                  </div>
                  <div
                    className="flex items-center gap-1 text-[10px] text-white/70"
                    title={`CPU: ${Math.round(metrics.cpu_percent)}%`}
                  >
                    <Activity className="w-3 h-3 text-red-400" />
                    <span className="font-mono">{Math.round(metrics.cpu_percent)}%</span>
                  </div>
                  <div className="flex items-center gap-1 text-[10px] text-[#39ff14]" title="GPU: ON">
                    <Zap className="w-3 h-3 fill-[#39ff14]/20" />
                  </div>
                </>
              )}
            </motion.div>
          )}
        </motion.div>

        {/* Center — Identity */}
        {!isModalOpen && (
          <motion.div
            layout
            transition={spring}
            className="absolute pointer-events-auto top-3 sm:top-4 left-1/2 -translate-x-1/2 z-[40] hidden lg:block"
          >
          <div className="relative group">
            <motion.div
              layout
              transition={spring}
              className="bg-[var(--surface-card)]/90 backdrop-blur-md border border-emerald-500/25 shadow-xs rounded-full px-3 py-1 flex items-center justify-center cursor-default shrink-0 min-h-[28px]"
            >
              <span className="text-emerald-600 dark:text-[#39ff14] font-mono tracking-widest text-[8px] uppercase font-bold">
                Xavier {liveVersion || (typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "0.2.20")}
              </span>
            </motion.div>

            <button
              type="button"
              onClick={() => setShowConfig(!showConfig)}
              className="absolute -right-7 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity text-[var(--foreground-muted)] hover:text-emerald-500 p-1.5 outline-none cursor-pointer"
              title="Configure Status Bar"
              aria-label="Configure Status Bar"
            >
              <Settings className="w-3.5 h-3.5 hover:animate-[spin_4s_linear_infinite]" aria-hidden="true" />
            </button>

            <AnimatePresence>
              {showConfig && (
                <motion.div
                  initial={{ opacity: 0, y: 10, scale: 0.95 }}
                  animate={{ opacity: 1, y: 0, scale: 1 }}
                  exit={{ opacity: 0, y: 10, scale: 0.95 }}
                  className="absolute top-full mt-2 left-1/2 -translate-x-1/2 w-48 max-w-[calc(100vw-2rem)] bg-[var(--surface-card)] backdrop-blur-xl border border-[var(--border-subtle)] rounded-xl p-3 shadow-2xl flex flex-col gap-2 z-[60] text-[var(--foreground)]"
                >
                  <h3 className="text-[10px] uppercase tracking-widest text-[var(--foreground-muted)] mb-1 px-1">Modules</h3>
                  {Object.entries({
                    time: "Time & Date",
                    founder: "Founder Node",
                    marketplace: "Marketplace Status",
                    resources: "System Resources",
                    channels: "Communication",
                    security: "Security & Proxy",
                    sync: "Node Sync",
                    ai: "AI Providers",
                    notifications: "Notifications",
                  }).map(([key, label]) => (
                    <button
                      type="button"
                      key={key}
                      onClick={() => toggleModule(key as keyof typeof modules)}
                      className="flex items-center justify-between px-2 py-1.5 hover:bg-[var(--surface-hover)] rounded-lg transition-colors group/btn outline-none cursor-pointer"
                      aria-label={`Toggle ${label} module`}
                    >
                      <span className="text-xs text-[var(--foreground)] font-mono">{label}</span>
                      <div className={`w-3 h-3 rounded-sm border flex items-center justify-center transition-colors ${modules[key as keyof typeof modules] ? "bg-emerald-500/20 border-emerald-500/50" : "border-[var(--border-subtle)] group-hover/btn:border-[var(--foreground-muted)]"}`}>
                        {modules[key as keyof typeof modules] && <div className="w-1.5 h-1.5 bg-emerald-500 rounded-[1px]" />}
                      </div>
                    </button>
                  ))}
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </motion.div>
        )}

        {/* Right Group */}
        <motion.div
          layout
          transition={spring}
          className={`flex gap-1.5 pointer-events-auto ${isModalOpen ? "absolute right-2 lg:right-4 top-1/2 -translate-y-1/2 flex-col items-end z-[60]" : "absolute right-2 sm:right-3 md:right-4 top-3 sm:top-4 flex-row items-center z-[70]"}`}
        >
          {/* Active Node Indicator */}
          <motion.button
            layout
            transition={spring}
            type="button"
            onClick={() => setShowNodeModal(true)}
            className={`bg-[var(--surface-card)]/80 backdrop-blur-md border ${
              nodeStatus === "connected"
                ? "border-emerald-500/30 hover:border-emerald-400/50"
                : "border-amber-500/50 hover:border-amber-400/80 bg-amber-500/10"
            } shadow-md rounded-full px-2.5 py-1 flex items-center gap-1.5 h-7 shrink-0 transition-colors cursor-pointer`}
            title={`Node: ${remoteUrl || "Local"} | Status: ${nodeStatus}${retryCount > 0 ? ` (Retries: ${retryCount})` : ""}`}
            aria-label="Node Connection Settings"
          >
            {nodeStatus === "connected" ? (
              <Server className="w-3 h-3 text-emerald-500 dark:text-emerald-400" />
            ) : (
              <WifiOff className="w-3 h-3 text-amber-500 dark:text-amber-400 animate-pulse" />
            )}
            <span className={`font-mono text-[9px] uppercase tracking-wide hidden sm:inline-block ${nodeStatus === "connected" ? "text-emerald-600 dark:text-emerald-300" : "text-amber-600 dark:text-amber-300 font-bold"}`}>
              {remoteUrl ? "Remote Node" : "Local Node"}
            </span>
            {nodeStatus !== "connected" && (
              <span className="font-mono text-[8px] bg-amber-500/20 text-amber-600 dark:text-amber-300 border border-amber-500/40 px-1 rounded animate-pulse">
                Retry #{retryCount}
              </span>
            )}
          </motion.button>

          {/* Maloca ops workspace (Desktop / Tablet) */}
          <motion.button
            layout
            transition={spring}
            type="button"
            onClick={() => { window.location.hash = "#/maloca"; }}
            className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-[var(--border-subtle)] shadow-md rounded-full px-2.5 py-1 hidden md:flex items-center gap-1.5 h-7 shrink-0 hover:border-emerald-400/30 hover:bg-emerald-500/5 transition-colors cursor-pointer text-[var(--foreground)]"
            title="Abrir Maloca (ops workspace)"
            aria-label="Abrir Maloca (ops workspace)"
          >
            <Home className="w-3 h-3 text-emerald-600 dark:text-emerald-300" />
            <span className="font-mono text-[9px] text-emerald-700 dark:text-emerald-200 uppercase tracking-wide hidden lg:inline-block">
              Maloca
            </span>
          </motion.button>

          {/* Settings / Control Node Button */}
          {onOpenConfig && (
            <motion.button
              layout
              transition={spring}
              type="button"
              onClick={onOpenConfig}
              className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-[var(--border-subtle)] shadow-md rounded-full px-2.5 py-1 flex items-center gap-1.5 h-7 shrink-0 hover:border-emerald-400/50 hover:bg-emerald-500/10 transition-colors text-[var(--foreground)]/80 hover:text-[var(--foreground)] cursor-pointer"
              title="Control Node Settings"
              aria-label="Control Node Settings"
              data-testid="topbar-settings-btn"
            >
              <Settings className="w-3.5 h-3.5 text-emerald-500 dark:text-emerald-400" />
              <span className="font-mono text-[9px] uppercase tracking-wide hidden sm:inline-block">
                Settings
              </span>
            </motion.button>
          )}

          {/* Marketplace Status Card Pill (Desktop) */}
          {modules.marketplace && (
            <div className="relative hidden md:block">
              <motion.button
                layout
                transition={spring}
                type="button"
                onClick={() => setShowMarketplaceCard((prev) => !prev)}
                className={`bg-[var(--surface-card)]/80 backdrop-blur-md border ${
                  showMarketplaceCard
                    ? "border-emerald-500/60 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300"
                    : "border-[var(--border-subtle)] text-[var(--foreground)]/80 hover:border-emerald-400/30 hover:bg-emerald-500/5 hover:text-[var(--foreground)]"
                } shadow-md rounded-full px-2.5 py-1 flex items-center gap-1.5 h-7 shrink-0 transition-colors cursor-pointer`}
                title="Marketplace v2 Status"
                aria-label="Marketplace v2 Status"
              >
                <ShoppingBag className="w-3 h-3 text-emerald-500 dark:text-emerald-400" />
                <span className="font-mono text-[9px] uppercase tracking-wide hidden lg:inline-block">
                  Market
                </span>
              </motion.button>

              <AnimatePresence>
                {showMarketplaceCard && (
                  <motion.div
                    initial={{ opacity: 0, y: 10, scale: 0.95 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, y: 10, scale: 0.95 }}
                    className="absolute top-full mt-2 right-0 w-72 max-w-[calc(100vw-2rem)] z-[70]"
                  >
                    <MarketplaceStatusCard
                      onOpenMarketplace={() => {
                        setShowMarketplaceCard(false);
                        window.location.hash = "#/maloca";
                      }}
                    />
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )}

          {/* Security & Proxy (Desktop) */}
          {modules.security && (
            <motion.div
              layout
              transition={spring}
              className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-[var(--border-subtle)] shadow-md rounded-full px-2.5 py-1 hidden md:flex items-center gap-2 h-7 shrink-0 text-[var(--foreground)]"
            >
              <div className="flex items-center gap-1" title="TPM HW Encryption Active">
                <ShieldCheck className="w-3 h-3 text-emerald-500 dark:text-emerald-400" />
              </div>
              <div className="w-px h-2.5 bg-[var(--border-subtle)]" />
              <div className="flex items-center gap-1" title="API Token Proxy">
                <Key className="w-3 h-3 text-amber-500" />
              </div>
            </motion.div>
          )}

          {/* Node Sync Pill (Desktop) */}
          {modules.sync && (
            <div className="relative hidden md:block">
              <motion.button
                layout
                transition={spring}
                type="button"
                onClick={() => setShowSyncPopover((prev) => !prev)}
                className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-cyan-500/30 hover:border-cyan-400/60 shadow-md rounded-full px-2.5 py-1 flex items-center gap-1.5 h-7 shrink-0 sm:flex cursor-pointer transition-colors"
                title="Cloud & Peer Memory Synchronization Status"
                aria-label="Memory Sync Control & Status"
              >
                <Wifi className={`w-3 h-3 text-cyan-500 dark:text-cyan-400 ${isSyncing ? "animate-pulse text-cyan-400" : ""}`} />
                <span className="font-mono text-[9px] text-cyan-600 dark:text-cyan-300 uppercase tracking-wide hidden lg:inline-block">
                  Sync
                </span>
                <div className="w-8 h-0.5 bg-[var(--surface)] rounded-full overflow-hidden border border-[var(--border-subtle)] mx-0.5 hidden xl:block">
                  <div className={`h-full bg-cyan-500 dark:bg-cyan-400 w-[98%] shadow-[0_0_6px_rgba(34,211,238,0.5)] ${isSyncing ? "animate-pulse" : ""}`} />
                </div>
                {isSyncing ? (
                  <RefreshCw className="w-2.5 h-2.5 text-cyan-500 dark:text-cyan-300 animate-spin" aria-label="Syncing progress indicator" />
                ) : (
                  <span className="font-mono text-[9px] text-cyan-600 dark:text-cyan-400 font-bold">
                    {syncLatencyMs !== null ? `${syncLatencyMs}ms` : "98%"}
                  </span>
                )}
              </motion.button>

              <SyncStatusPopover
                isOpen={showSyncPopover}
                onClose={() => setShowSyncPopover(false)}
                isSyncing={isSyncing}
                syncLatencyMs={syncLatencyMs}
                lastSyncTime={lastSyncTime}
                onTriggerSync={handleTriggerSync}
              />
            </div>
          )}

          {/* AI Providers Pill (Desktop) */}
          {modules.ai && (config.has_gemini || config.has_openai) && (
            <motion.div
              layout
              transition={spring}
              className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-[var(--border-subtle)] shadow-md rounded-full px-2.5 py-1 hidden md:flex items-center gap-1.5 h-7 shrink-0"
            >
              <Bot className="w-3 h-3 text-purple-500 dark:text-purple-400" />
              <div className="flex items-center gap-1">
                {config.has_gemini && (
                  <span className="hidden lg:inline-block text-[8px] bg-blue-500/15 text-blue-600 dark:text-blue-300 border border-blue-500/25 px-1 py-px rounded uppercase tracking-wider font-mono">
                    GEM
                  </span>
                )}
                {config.has_openai && (
                  <span className="hidden lg:inline-block text-[8px] bg-emerald-500/15 text-emerald-600 dark:text-emerald-300 border border-emerald-500/25 px-1 py-px rounded uppercase tracking-wider font-mono">
                    OAI
                  </span>
                )}
              </div>
            </motion.div>
          )}

          {/* Mobile Secondary Menu Dropdown (Collapsed < 768px) */}
          <div className="relative md:hidden">
            <motion.button
              layout
              transition={spring}
              type="button"
              onClick={() => setShowMobileMenu((prev) => !prev)}
              className={`bg-[var(--surface-card)]/80 backdrop-blur-md border ${
                showMobileMenu
                  ? "border-emerald-500/50 bg-emerald-500/15 text-emerald-600 dark:text-emerald-300"
                  : "border-[var(--border-subtle)] text-[var(--foreground)]/80 hover:text-[var(--foreground)] hover:border-[var(--border-focus)]"
              } shadow-md rounded-full h-7 w-7 flex items-center justify-center shrink-0 transition-colors cursor-pointer`}
              title="More Actions & Telemetry"
              aria-label="More Actions & Telemetry"
              aria-expanded={showMobileMenu}
            >
              <SlidersHorizontal className="w-3 h-3" />
            </motion.button>

            <AnimatePresence>
              {showMobileMenu && (
                <motion.div
                  initial={{ opacity: 0, y: 8, scale: 0.95 }}
                  animate={{ opacity: 1, y: 0, scale: 1 }}
                  exit={{ opacity: 0, y: 8, scale: 0.95 }}
                  className="absolute top-full mt-2 right-0 w-64 max-w-[calc(100vw-2rem)] bg-[var(--surface-card)] backdrop-blur-xl border border-[var(--border-subtle)] rounded-2xl p-2.5 shadow-2xl z-[80] space-y-2 text-xs font-mono text-[var(--foreground)]"
                >
                  <div className="px-2 py-1 text-[10px] uppercase tracking-wider text-[var(--foreground-muted)] font-bold border-b border-[var(--border-subtle)] flex items-center justify-between">
                    <span>Xavier Telemetry</span>
                    <span className="text-emerald-600 dark:text-[#39ff14] text-[9px]">ONLINE</span>
                  </div>

                  <button
                    type="button"
                    onClick={() => {
                      setShowMobileMenu(false);
                      window.location.hash = "#/maloca";
                    }}
                    className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-xl bg-[var(--surface)] hover:bg-emerald-500/10 hover:border-emerald-500/30 border border-transparent transition-colors text-[var(--foreground)] cursor-pointer"
                  >
                    <div className="flex items-center gap-2">
                      <Home className="w-3.5 h-3.5 text-emerald-500 dark:text-emerald-400" />
                      <span>Maloca Ops</span>
                    </div>
                    <span className="text-[10px] text-[var(--foreground-muted)]">#/maloca</span>
                  </button>

                  <div className="p-2 rounded-xl bg-[var(--surface)] border border-[var(--border-subtle)] space-y-1.5">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-1.5 text-cyan-600 dark:text-cyan-400">
                        <Wifi className="w-3 h-3" />
                        <span className="text-[11px] font-semibold">Sync</span>
                      </div>
                      <span className="text-[10px] text-cyan-600 dark:text-cyan-300">
                        {syncLatencyMs !== null ? `${syncLatencyMs}ms` : "98%"}
                      </span>
                    </div>
                    <button
                      type="button"
                      onClick={() => {
                        handleTriggerSync();
                        setShowMobileMenu(false);
                      }}
                      disabled={isSyncing}
                      className="w-full py-1 px-2 bg-cyan-500/15 hover:bg-cyan-500/25 border border-cyan-500/30 text-cyan-600 dark:text-cyan-300 rounded-lg text-[10px] flex items-center justify-center gap-1.5 transition-colors disabled:opacity-50 cursor-pointer shadow-xs"
                    >
                      <RefreshCw className={`w-2.5 h-2.5 ${isSyncing ? "animate-spin" : ""}`} />
                      <span>{isSyncing ? "Sincronizando..." : "Sincronizar"}</span>
                    </button>
                  </div>

                  <div className="flex items-center justify-between px-2.5 py-1.5 rounded-xl bg-[var(--surface)] text-[var(--foreground)] border border-[var(--border-subtle)]">
                    <div className="flex items-center gap-2">
                      <ShieldCheck className="w-3.5 h-3.5 text-emerald-500 dark:text-emerald-400" />
                      <span>TPM Crypto</span>
                    </div>
                    <span className="text-[9px] text-emerald-600 dark:text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-1.5 py-0.5 rounded">
                      ACTIVE
                    </span>
                  </div>

                  {modules.ai && (config.has_gemini || config.has_openai) && (
                    <div className="flex items-center justify-between px-2.5 py-1.5 rounded-xl bg-[var(--surface)] text-[var(--foreground)] border border-[var(--border-subtle)]">
                      <div className="flex items-center gap-2">
                        <Bot className="w-3.5 h-3.5 text-purple-500 dark:text-purple-400" />
                        <span>AI Engines</span>
                      </div>
                      <div className="flex items-center gap-1">
                        {config.has_gemini && (
                          <span className="text-[8px] bg-blue-500/20 text-blue-600 dark:text-blue-300 border border-blue-500/30 px-1 rounded">
                            GEM
                          </span>
                        )}
                        {config.has_openai && (
                          <span className="text-[8px] bg-emerald-500/20 text-emerald-600 dark:text-emerald-300 border border-emerald-500/30 px-1 rounded">
                            OAI
                          </span>
                        )}
                      </div>
                    </div>
                  )}

                  {modules.marketplace && (
                    <button
                      type="button"
                      onClick={() => {
                        setShowMobileMenu(false);
                        window.location.hash = "#/maloca";
                      }}
                      className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-xl bg-[var(--surface)] hover:bg-emerald-500/10 text-[var(--foreground)] border border-[var(--border-subtle)] transition-colors cursor-pointer"
                    >
                      <div className="flex items-center gap-2">
                        <ShoppingBag className="w-3.5 h-3.5 text-emerald-500 dark:text-emerald-400" />
                        <span>Marketplace</span>
                      </div>
                      <span className="text-[10px] text-emerald-600 dark:text-emerald-400">Explorar</span>
                    </button>
                  )}
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Notifications Bell */}
          {modules.notifications && (
            <div className="relative">
              <motion.button
                ref={bellRef}
                layout
                transition={spring}
                onClick={() => setShowNotifications((prev) => !prev)}
                aria-label="Notifications"
                aria-expanded={showNotifications}
                className="bg-[var(--surface-card)]/80 backdrop-blur-md border border-[var(--border-subtle)] shadow-md rounded-full px-2 hover:bg-[var(--surface-hover)] hover:border-[var(--border-focus)] transition-all flex items-center justify-center h-7 w-7 shrink-0 cursor-pointer text-[var(--foreground)]"
                title={`${memoryCount} Memories | ${streamUnreadCount} Unread`}
              >
                <div className="relative flex items-center justify-center">
                  <Bell className="w-3.5 h-3.5 text-[var(--foreground)]/70" />
                  {streamUnreadCount > 0 && (
                    <div className="absolute -top-1.5 -right-1.5 bg-red-500 text-white text-[7px] font-bold px-1 rounded-full border border-[var(--surface-card)] min-w-[13px] text-center shadow-[0_0_5px_rgba(239,68,68,0.4)]">
                      {streamUnreadCount > 9 ? "9+" : streamUnreadCount}
                    </div>
                  )}
                </div>
              </motion.button>

              <AnimatePresence>
                {showNotifications && (
                  <NotificationsDropdown onClose={() => setShowNotifications(false)} />
                )}
              </AnimatePresence>
            </div>
          )}
        </motion.div>
      </div>

      {/* Node Connection Modal Overlay */}
      <NodeConnectionModal
        isOpen={showNodeModal}
        onClose={() => setShowNodeModal(false)}
        nodeStatus={nodeStatus}
        retryCount={retryCount}
        remoteUrl={remoteUrl}
        onSaveRemoteUrl={handleSaveRemoteUrl}
      />

      {/* Messaging Config Modal */}
      <AnimatePresence>
        {showMessaging && (
          <MessagingConfigModal
            initialTab={messagingTab}
            onClose={() => setShowMessaging(false)}
          />
        )}
      </AnimatePresence>
    </>
  );
});
