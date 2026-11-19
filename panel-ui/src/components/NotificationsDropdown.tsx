import {
	Activity,
	AlertTriangle,
	Bot,
	Brain,
	CheckCheck,
	Clock,
	RefreshCw,
	X,
	Zap,
} from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import React, { useMemo, useState } from "react";
import { useNotificationStream } from "../hooks/useNotificationStream";

export interface Notification {
	id: string;
	islandId: IslandId;
	island_id?: IslandId; // Backend uses snake_case
	title: string;
	body: string;
	timestamp: Date | string;
	read: boolean;
	severity?: "info" | "warning" | "error" | "success";
}

type IslandId = "system" | "memory" | "agents" | "errors";

interface Island {
	id: IslandId;
	label: string;
	icon: React.ReactNode;
	color: string;
	bgColor: string;
	borderColor: string;
}

const ISLANDS: Island[] = [
	{
		id: "system",
		label: "System",
		icon: <Activity className="w-3 h-3" />,
		color: "text-cyan-600 dark:text-cyan-400",
		bgColor: "bg-cyan-500/10",
		borderColor: "border-cyan-500/20",
	},
	{
		id: "memory",
		label: "Memory",
		icon: <Brain className="w-3 h-3" />,
		color: "text-emerald-600 dark:text-emerald-400",
		bgColor: "bg-emerald-500/10",
		borderColor: "border-emerald-500/20",
	},
	{
		id: "agents",
		label: "Agents",
		icon: <Bot className="w-3 h-3" />,
		color: "text-purple-600 dark:text-purple-400",
		bgColor: "bg-purple-500/10",
		borderColor: "border-purple-500/20",
	},
	{
		id: "errors",
		label: "Errors",
		icon: <AlertTriangle className="w-3 h-3" />,
		color: "text-rose-600 dark:text-rose-400",
		bgColor: "bg-rose-500/10",
		borderColor: "border-rose-500/20",
	},
];

function formatRelativeTime(dateInput?: Date | string | null): string {
	if (!dateInput) return "just now";
	const date = typeof dateInput === "string" ? new Date(dateInput) : dateInput;
	if (!date || isNaN(date.getTime())) return "just now";
	const diff = (Date.now() - date.getTime()) / 1000;
	if (diff < 60) return "just now";
	if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
	if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
	return `${Math.floor(diff / 86400)}d ago`;
}

function renderBodyWithLinks(body: string) {
	if (!body) return null;
	const urlRegex = /(https?:\/\/[^\s]+)/g;
	const parts = body.split(urlRegex);

	return parts.map((part, index) => {
		if (urlRegex.test(part)) {
			const label = part.includes("/commit/")
				? "Ver detalles del commit →"
				: part.includes("/releases/")
				? "Ver release notes →"
				: part.length > 45
				? `${part.slice(0, 42)}...`
				: part;

			return (
				<a
					key={`link-${index}`}
					href={part}
					target="_blank"
					rel="noopener noreferrer"
					className="text-emerald-500 hover:text-emerald-400 dark:text-emerald-400 dark:hover:text-emerald-300 underline font-medium cursor-pointer break-all"
					onClick={(e) => e.stopPropagation()}
				>
					{label}
				</a>
			);
		}
		return part;
	});
}

/**
 * ⚡ Bolt Performance Optimization
 *
 * 💡 What: Extracted notification list item into a memoized component wrapper.
 * 🎯 Why: When changing the active island tab in NotificationsDropdown, all NotificationItem components would re-render due to the inline map causing O(N) operations.
 * 📊 Impact: Prevents O(N) re-renders for unchanged notification items during UI interactions (e.g. clicking tabs or marking a single notification as read).
 */
const NotificationItem = React.memo(function NotificationItem({
	notif,
	onRead,
}: {
	notif: Notification;
	onRead: (id: string) => void;
}) {
	const severityDot = {
		success: "bg-green-400",
		info: "bg-cyan-400",
		warning: "bg-amber-400",
		error: "bg-red-400",
	}[notif.severity || "info"];

	return (
		<motion.div
			layout
			initial={{ opacity: 0, y: -4 }}
			animate={{ opacity: 1, y: 0 }}
			exit={{ opacity: 0, x: 20, height: 0 }}
			className={`flex gap-2.5 p-2.5 rounded-lg transition-colors cursor-default group ${
				notif.read ? "bg-transparent" : "bg-[var(--surface)]/50"
			} hover:bg-[var(--surface-hover)]`}
		>
			<div className="flex-shrink-0 mt-0.5">
				<div
					className={`w-1.5 h-1.5 rounded-full ${severityDot} ${notif.read ? "opacity-30" : ""}`}
				/>
			</div>
			<div className="flex-1 min-w-0">
				<div className="flex items-center justify-between gap-2">
					<p
						className={`text-[11px] font-medium truncate ${notif.read ? "text-[var(--foreground-muted)] opacity-60" : "text-[var(--foreground)]"}`}
					>
						{notif.title}
					</p>
					<span className="flex-shrink-0 text-[9px] text-[var(--foreground-muted)]/75 flex items-center gap-1">
						<Clock className="w-2.5 h-2.5" />
						{formatRelativeTime(notif.timestamp)}
					</span>
				</div>
				<p
					className={`text-[10px] mt-0.5 leading-relaxed whitespace-pre-line ${notif.read ? "text-[var(--foreground-muted)]/60" : "text-[var(--foreground-muted)]"}`}
				>
					{renderBodyWithLinks(notif.body)}
				</p>
			</div>
			{!notif.read && (
				<button
					type="button"
					onClick={() => onRead(notif.id)}
					aria-label="Mark as read"
					className="flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity p-0.5 text-[var(--foreground-muted)] hover:text-[var(--foreground)] cursor-pointer"
				>
					<X className="w-3 h-3" aria-hidden="true" />
				</button>
			)}
		</motion.div>
	);
});

interface NotificationsDropdownProps {
	onClose: () => void;
	anchorRef?: React.RefObject<HTMLElement>;
	isLoading?: boolean;
}

export default React.memo(function NotificationsDropdown({
	onClose,
	isLoading: propIsLoading,
}: NotificationsDropdownProps) {
	const {
		notifications,
		unreadCount,
		isLoading: streamLoading,
		markRead,
		markAllRead,
	} = useNotificationStream();

	const [activeIsland, setActiveIsland] = useState<IslandId | "all">("all");
	const isLoading = propIsLoading !== undefined ? propIsLoading : streamLoading;

	const filtered = useMemo(() => {
		return activeIsland === "all"
			? notifications
			: notifications.filter(
					(n) => (n.island_id || n.islandId) === activeIsland,
				);
	}, [activeIsland, notifications]);

	const islandCounts = useMemo(() => {
		const counts = ISLANDS.reduce(
			(acc, island) => {
				acc[island.id] = 0;
				return acc;
			},
			{} as Record<IslandId, number>,
		);

		for (const n of notifications) {
			if (!n.read) {
				const id = (n.island_id || n.islandId) as IslandId;
				if (id && counts[id] !== undefined) {
					counts[id]++;
				}
			}
		}
		return counts;
	}, [notifications]);

	return (
		<>
			{/* Backdrop */}
			<div
				className="fixed inset-0 z-[65]"
				onClick={onClose}
				aria-hidden="true"
			/>

			<motion.div
				initial={{ opacity: 0, y: -8, scale: 0.97 }}
				animate={{ opacity: 1, y: 0, scale: 1 }}
				exit={{ opacity: 0, y: -8, scale: 0.97 }}
				transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
				className="absolute right-0 top-full mt-2 w-[calc(100vw-2rem)] sm:w-80 max-w-[340px] max-h-[420px] bg-[var(--surface-card)] border border-[var(--border-subtle)] rounded-xl shadow-2xl flex flex-col overflow-hidden z-[66]"
				style={{ top: "100%" }}
			>
				{/* Header */}
				<div className="flex items-center justify-between px-4 py-3 border-b border-[var(--border-subtle)]">
					<div className="flex items-center gap-2">
						<span className="text-xs font-semibold text-[var(--foreground)] tracking-wide">
							Notifications
						</span>
						{unreadCount > 0 && (
							<span className="px-1.5 py-0.5 bg-emerald-500/15 border border-emerald-500/30 text-emerald-600 dark:text-emerald-400 text-[9px] font-bold rounded-full">
								{unreadCount}
							</span>
						)}
					</div>
					<div className="flex items-center gap-1">
						{unreadCount > 0 && (
							<button
								type="button"
								onClick={markAllRead}
								className="flex items-center gap-1 px-2 py-1 text-[9px] text-[var(--foreground-muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-hover)] rounded-lg transition-all cursor-pointer"
							>
								<CheckCheck className="w-3 h-3" aria-hidden="true" />
								All read
							</button>
						)}
						<button
							type="button"
							onClick={onClose}
							aria-label="Close notifications"
							className="p-1 text-[var(--foreground-muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-hover)] rounded-lg transition-all cursor-pointer"
						>
							<X className="w-3.5 h-3.5" aria-hidden="true" />
						</button>
					</div>
				</div>

				{/* Island filter tabs */}
				<div className="flex gap-1 px-3 py-2 border-b border-[var(--border-subtle)] overflow-x-auto no-scrollbar">
					<button
						type="button"
						onClick={() => setActiveIsland("all")}
						className={`flex-shrink-0 flex items-center gap-1 px-2 py-1 rounded-md text-[9px] uppercase tracking-widest transition-all cursor-pointer ${
							activeIsland === "all"
								? "bg-[var(--surface)] text-[var(--foreground)] font-semibold border border-[var(--border-subtle)] shadow-xs"
								: "text-[var(--foreground-muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-hover)]"
						}`}
					>
						<RefreshCw className="w-2.5 h-2.5" />
						All
					</button>
					{ISLANDS.map((island) => (
						<button
							type="button"
							key={island.id}
							onClick={() => setActiveIsland(island.id)}
							className={`flex-shrink-0 flex items-center gap-1 px-2 py-1 rounded-md text-[9px] uppercase tracking-widest transition-all relative cursor-pointer ${
								activeIsland === island.id
									? `${island.bgColor} ${island.color} border ${island.borderColor} font-semibold shadow-xs`
									: "text-[var(--foreground-muted)] hover:text-[var(--foreground)] hover:bg-[var(--surface-hover)]"
							}`}
						>
							{island.icon}
							{island.label}
							{islandCounts[island.id] > 0 && (
								<span
									className={`ml-0.5 px-1 py-px rounded-full text-[8px] font-bold ${island.bgColor} ${island.color}`}
								>
									{islandCounts[island.id]}
								</span>
							)}
						</button>
					))}
				</div>

				{/* Notifications list */}
				<div className="flex-1 overflow-y-auto p-2">
					<AnimatePresence>
						{isLoading ? (
							<div className="space-y-2" data-testid="notification-skeletons">
								<div className="h-12 animate-pulse bg-[var(--surface)]/60 rounded-lg" />
								<div className="h-12 animate-pulse bg-[var(--surface)]/60 rounded-lg" />
								<div className="h-12 animate-pulse bg-[var(--surface)]/60 rounded-lg" />
							</div>
						) : filtered.length === 0 ? (
							<motion.div
								initial={{ opacity: 0 }}
								animate={{ opacity: 1 }}
								className="flex flex-col items-center justify-center py-8 text-[var(--foreground-muted)]"
							>
								<Zap className="w-6 h-6 mb-2 opacity-30" />
								<p className="text-[11px]">No notifications</p>
							</motion.div>
						) : (
							filtered.map((n) => (
								<NotificationItem key={n.id} notif={n} onRead={markRead} />
							))
						)}
					</AnimatePresence>
				</div>

				{/* Footer */}
				<div className="px-4 py-2 border-t border-[var(--border-subtle)] bg-[var(--surface)]">
					<p className="text-[9px] text-[var(--foreground-muted)]/70 text-center font-mono">
						Persistent notification system active
					</p>
				</div>
			</motion.div>
		</>
	);
});
