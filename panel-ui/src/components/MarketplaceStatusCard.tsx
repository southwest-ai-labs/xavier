import React, { useEffect, useState } from "react";
import { ShoppingBag, ArrowRight, RefreshCw } from "lucide-react";
import { malocaApi } from "../maloca/api";

// The node whose consent tier this card shows. Matches the default node in MarketplaceView.
const CONSENT_NODE_ID = "local-node";

export interface MarketplaceStatusCardProps {
  onOpenMarketplace?: () => void;
  className?: string;
}

export function MarketplaceStatusCard({
  onOpenMarketplace,
  className = "",
}: MarketplaceStatusCardProps) {
  const [activeListingsCount, setActiveListingsCount] = useState<number | null>(null);
  const [consentStatus, setConsentStatus] = useState<string>("off");
  const [loading, setLoading] = useState(true);

  const fetchStatus = async () => {
    setLoading(true);
    try {
      const listings = await malocaApi.getMarketplaceListings().catch(() => []);
      const active = listings.filter((l) => !l.revoked_at_ms).length;
      setActiveListingsCount(active);

      const consent = await malocaApi.getMarketplaceConsent(CONSENT_NODE_ID).catch(() => null);
      if (consent) {
        setConsentStatus(consent.state || "off");
      }
    } catch {
      setActiveListingsCount(0);
      setConsentStatus("off");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void fetchStatus();
  }, []);

  const handleOpen = () => {
    if (onOpenMarketplace) {
      onOpenMarketplace();
    } else {
      window.location.hash = "#/maloca";
    }
  };

  return (
    <div
      data-testid="marketplace-status-card"
      className={`p-4 rounded-xl border border-[var(--border-subtle)] bg-[var(--surface-card)] text-[var(--foreground)] shadow-xl font-sans ${className}`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <ShoppingBag className="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
          <span className="text-xs font-mono font-bold tracking-wider uppercase text-[var(--foreground)]">
            Marketplace v2
          </span>
        </div>
        <button
          type="button"
          onClick={() => void fetchStatus()}
          className="text-[var(--foreground-muted)] hover:text-[var(--foreground)] transition-colors cursor-pointer"
          title="Refresh Status"
          aria-label="Refresh Status"
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : ""} />
        </button>
      </div>

      <div className="grid grid-cols-2 gap-2 mb-3">
        <div className="p-2.5 rounded-lg bg-[var(--surface)] border border-[var(--border-subtle)]">
          <div className="text-[10px] font-mono text-[var(--foreground-muted)] uppercase">Active Listings</div>
          <div data-testid="card-active-listings" className="text-lg font-bold text-[var(--foreground)] font-mono">
            {loading ? "..." : (activeListingsCount ?? 0)}
          </div>
        </div>
        <div className="p-2.5 rounded-lg bg-[var(--surface)] border border-[var(--border-subtle)]">
          <div className="text-[10px] font-mono text-[var(--foreground-muted)] uppercase">Consent Tier</div>
          <div className="text-xs font-bold font-mono uppercase text-emerald-600 dark:text-emerald-400 mt-1">
            {loading ? "..." : consentStatus}
          </div>
        </div>
      </div>

      <button
        type="button"
        onClick={handleOpen}
        className="w-full flex items-center justify-between px-3 py-2 bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-600 dark:text-emerald-300 border border-emerald-500/30 rounded-lg text-xs font-mono transition-all group cursor-pointer shadow-xs"
      >
        <span>Open Maloca Console</span>
        <ArrowRight size={13} className="group-hover:translate-x-0.5 transition-transform" />
      </button>
    </div>
  );
}
export default MarketplaceStatusCard;
