import { useEffect, useRef } from "react";
import { useAuthStore } from "../auth/AuthProvider";

const BASE_INTERVAL_MS = 15 * 60 * 1000; // 15 minutes
const MAX_INTERVAL_MS = 60 * 60 * 1000; // 60 minutes cap

/**
 * Schedules periodic session refresh with:
 * - Visibility awareness (pauses when tab is hidden)
 * - Online/offline awareness (skips when navigator.onLine is false)
 * - Exponential backoff on consecutive failures (doubles up to 60 min)
 */
export const useSession = () => {
  const refreshSession = useAuthStore((state) => state.refreshSession);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const failureCountRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!isAuthenticated) return;

    const scheduleNext = () => {
      const backoffMs = Math.min(
        BASE_INTERVAL_MS * Math.pow(2, failureCountRef.current),
        MAX_INTERVAL_MS,
      );
      timerRef.current = setTimeout(async () => {
        // Skip when tab is hidden or browser is offline — not a real failure
        if (document.hidden || !navigator.onLine) {
          scheduleNext();
          return;
        }
        try {
          await refreshSession();
          failureCountRef.current = 0;
        } catch {
          failureCountRef.current += 1;
        }
        scheduleNext();
      }, backoffMs);
    };

    scheduleNext();

    // Resume immediately when the tab becomes visible again
    const onVisibilityChange = () => {
      if (!document.hidden && isAuthenticated) {
        if (timerRef.current) clearTimeout(timerRef.current);
        failureCountRef.current = 0;
        scheduleNext();
      }
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [isAuthenticated, refreshSession]);

  return { refreshSession, isAuthenticated };
};
