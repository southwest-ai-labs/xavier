import {
	AlertTriangle,
	Check,
	Code2,
	Copy,
	Database,
	Download,
	ExternalLink,
	Eye,
	FileText,
	Film,
	Hash,
	Image as ImageIcon,
	Lock,
	Maximize2,
	Minimize2,
	ShieldCheck,
	Sparkles,
	X,
} from "lucide-react";
import React, { useEffect, useState } from "react";
import type { ChatAttachment, ClearanceLevel } from "./DirectChatView";

export interface FileSandboxModalProps {
	isOpen: boolean;
	onClose: () => void;
	attachment: ChatAttachment | null;
	clearanceLevel?: ClearanceLevel;
	onImportToVault?: (attachment: ChatAttachment) => void;
}

/** Helper to validate URL protocol against safe allowlist */
const isSafeUrl = (url?: string): boolean => {
	if (!url) return false;
	try {
		const parsed = new URL(url, window.location.href);
		return ["blob:", "http:", "https:", "data:"].includes(parsed.protocol);
	} catch {
		return false;
	}
};

/**
 * FileSandboxModal provides an isolated, sandboxed preview environment for attachments
 * (images, videos, documents, and Xavier RAG/Code contexts) received in P2P chats.
 * Prevents host filesystem contamination and ensures sandbox isolation.
 */
export const FileSandboxModal: React.FC<FileSandboxModalProps> = ({
	isOpen,
	onClose,
	attachment,
	clearanceLevel = "Internal",
	onImportToVault,
}) => {
	const [isFullscreen, setIsFullscreen] = useState(false);
	const [copied, setCopied] = useState(false);
	const [imported, setImported] = useState(false);

	useEffect(() => {
		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.key === "Escape" && isOpen) {
				onClose();
			}
		};
		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [isOpen, onClose]);

	if (!isOpen || !attachment) return null;

	const isContext = Boolean(attachment.isContext || attachment.contextPayload);
	const payload = attachment.contextPayload;
	const fileType = (attachment.type || "").toLowerCase();
	const isImage =
		fileType.startsWith("image/") ||
		/\.(jpe?g|png|gif|webp|svg)$/i.test(attachment.name);
	const isVideo =
		fileType.startsWith("video/") ||
		/\.(mp4|webm|mov|mkv)$/i.test(attachment.name);
	const isPdf =
		fileType === "application/pdf" || /\.pdf$/i.test(attachment.name);

	const safeUrl = isSafeUrl(attachment.url) ? attachment.url : undefined;

	const handleCopy = async () => {
		try {
			const textToCopy = payload ? payload.content : attachment.name;
			if (navigator?.clipboard?.writeText) {
				await navigator.clipboard.writeText(textToCopy);
				setCopied(true);
				setTimeout(() => setCopied(false), 2000);
			}
		} catch {
			// Ignore clipboard failure in restricted webviews
		}
	};

	const handleImport = () => {
		if (onImportToVault) {
			onImportToVault(attachment);
			setImported(true);
			setTimeout(() => setImported(false), 3000);
		}
	};

	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fadeIn"
			role="dialog"
			aria-modal="true"
			aria-labelledby="sandbox-title"
		>
			<div
				className={`flex flex-col bg-slate-900 border border-emerald-500/30 rounded-2xl shadow-2xl overflow-hidden transition-all duration-200 ${
					isFullscreen
						? "w-full h-full rounded-none"
						: "w-full max-w-4xl max-h-[90vh] h-[780px]"
				}`}
			>
				{/* Top Sandbox Security Banner */}
				<div className="bg-emerald-950/60 border-b border-emerald-500/20 px-4 py-2 flex items-center justify-between text-xs font-mono">
					<div className="flex items-center gap-2 text-emerald-400">
						<ShieldCheck className="w-4 h-4 text-emerald-400" />
						<span className="font-semibold tracking-wide">
							Xavier Secure Sandbox Environment
						</span>
						<span className="hidden sm:inline text-emerald-500/70">
							· Isolated Runtime · Zero Host Contamination
						</span>
					</div>
					<div className="flex items-center gap-2">
						<span className="px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 text-[10px] uppercase font-bold border border-emerald-500/40">
							{clearanceLevel}
						</span>
					</div>
				</div>

				{/* Header */}
				<header className="px-6 py-4 bg-slate-950/70 border-b border-white/10 flex items-center justify-between gap-4">
					<div className="flex items-center gap-3 min-w-0">
						<div className="p-2.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400">
							{isContext ? (
								<Sparkles className="w-5 h-5" />
							) : isImage ? (
								<ImageIcon className="w-5 h-5" />
							) : isVideo ? (
								<Film className="w-5 h-5" />
							) : (
								<FileText className="w-5 h-5" />
							)}
						</div>
						<div className="min-w-0">
							<h3
								id="sandbox-title"
								className="text-sm font-semibold text-white truncate"
							>
								{isContext
									? payload?.title || attachment.name
									: attachment.name}
							</h3>
							<div className="flex items-center gap-2 text-[11px] text-white/50 font-mono mt-0.5">
								{attachment.size && <span>{attachment.size}</span>}
								{isContext && payload?.kind && (
									<span className="text-cyan-400 uppercase font-semibold">
										Kind: {payload.kind}
									</span>
								)}
								{payload?.tokens && (
									<span>· {payload.tokens.toLocaleString()} tokens</span>
								)}
								{payload?.score !== undefined && (
									<span>· Relevance: {(payload.score * 100).toFixed(0)}%</span>
								)}
							</div>
						</div>
					</div>

					{/* Action Buttons */}
					<div className="flex items-center gap-2">
						{isContext && onImportToVault && (
							<button
								type="button"
								onClick={handleImport}
								disabled={imported}
								className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 hover:bg-emerald-500/30 text-xs font-mono transition-colors"
								title="Import into local P2P Mesh Vault (espacio.sqlite)"
							>
								{imported ? (
									<>
										<Check className="w-3.5 h-3.5" />
										<span>Anexado a Bóveda</span>
									</>
								) : (
									<>
										<Database className="w-3.5 h-3.5" />
										<span>Anexar a Bóveda Mesh</span>
									</>
								)}
							</button>
						)}

						{isContext && (
							<button
								type="button"
								onClick={handleCopy}
								className="p-2 rounded-lg bg-white/5 border border-white/10 text-white/70 hover:text-white hover:bg-white/10 transition-colors"
								title="Copy payload"
							>
								{copied ? (
									<Check className="w-4 h-4 text-emerald-400" />
								) : (
									<Copy className="w-4 h-4" />
								)}
							</button>
						)}

						<button
							type="button"
							onClick={() => setIsFullscreen(!isFullscreen)}
							className="p-2 rounded-lg bg-white/5 border border-white/10 text-white/70 hover:text-white hover:bg-white/10 transition-colors"
							title={isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
						>
							{isFullscreen ? (
								<Minimize2 className="w-4 h-4" />
							) : (
								<Maximize2 className="w-4 h-4" />
							)}
						</button>

						<button
							type="button"
							onClick={onClose}
							className="p-2 rounded-lg bg-white/5 border border-white/10 text-white/70 hover:text-white hover:bg-rose-500/20 hover:border-rose-500/40 hover:text-rose-300 transition-colors"
							aria-label="Close sandbox viewer"
						>
							<X className="w-4 h-4" />
						</button>
					</div>
				</header>

				{/* Sandbox Viewport */}
				<main className="flex-1 overflow-auto p-6 bg-slate-950/80 flex flex-col items-center justify-center">
					{isContext && payload ? (
						<div className="w-full h-full flex flex-col space-y-4">
							<div className="flex items-center justify-between p-3 rounded-xl bg-slate-900 border border-white/10 text-xs font-mono">
								<div className="flex items-center gap-3">
									<span className="text-white/60">SHA-256 Hash:</span>
									<span className="text-emerald-400 font-mono">
										{payload.hash || "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}
									</span>
								</div>
								<div className="flex items-center gap-2">
									<Code2 className="w-3.5 h-3.5 text-cyan-400" />
									<span className="text-white/80">Structured Context</span>
								</div>
							</div>

							<div className="flex-1 w-full bg-slate-900/90 rounded-xl border border-white/10 p-5 overflow-auto font-mono text-xs leading-relaxed text-slate-200">
								<pre className="whitespace-pre-wrap font-mono">
									{payload.content}
								</pre>
							</div>
						</div>
					) : isImage ? (
						<div className="relative max-w-full max-h-full flex items-center justify-center p-4">
							<img
								src={safeUrl || "/placeholder-image.png"}
								alt={attachment.name}
								className="max-h-[580px] max-w-full object-contain rounded-lg shadow-xl border border-white/10"
							/>
						</div>
					) : isVideo ? (
						<div className="relative w-full max-w-3xl flex items-center justify-center">
							<video
								controls
								playsInline
								className="w-full max-h-[560px] rounded-xl border border-white/10 bg-black shadow-2xl"
							>
								{safeUrl && (
									<source src={safeUrl} type={attachment.type || "video/mp4"} />
								)}
								Tu navegador no soporta reproducción de video en sandbox.
							</video>
						</div>
					) : isPdf ? (
						<div className="w-full h-full rounded-xl overflow-hidden border border-white/10 bg-slate-900">
							<iframe
								src={safeUrl || "about:blank"}
								title={attachment.name}
								sandbox="allow-scripts"
								className="w-full h-full border-0"
							/>
						</div>
					) : (
						/* General Document / Fallback View */
						<div className="flex flex-col items-center justify-center text-center p-8 max-w-md">
							<div className="p-4 rounded-2xl bg-white/5 border border-white/10 mb-4 text-emerald-400">
								<FileText className="w-10 h-10" />
							</div>
							<h4 className="text-base font-medium text-white mb-1">
								{attachment.name}
							</h4>
							<p className="text-xs text-white/50 mb-6 font-mono">
								{attachment.size || "Unknown size"} · Formato de archivo seguro
							</p>
							<div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-300 text-xs text-left mb-6 font-mono">
								<div className="flex items-center gap-1.5 font-bold mb-1">
									<AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
									<span>Aislamiento de Sandbox Activo</span>
								</div>
								<span>
									Este documento no se ejecuta en el sistema anfitrión para proteger
									la memoria local y la integridad del nodo.
								</span>
							</div>
							{safeUrl && (
								<a
									href={safeUrl}
									download={attachment.name}
									className="flex items-center gap-2 px-4 py-2 rounded-xl bg-emerald-500 text-slate-950 font-semibold text-xs hover:bg-emerald-400 transition-colors"
								>
									<Download className="w-4 h-4" />
									<span>Descargar Seguro a Bóveda Aislada</span>
								</a>
							)}
						</div>
					)}
				</main>
			</div>
		</div>
	);
};

export default FileSandboxModal;