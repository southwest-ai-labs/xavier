import {
	Brain,
	Check,
	Code2,
	FileCode,
	FileText,
	Filter,
	Layers,
	Lock,
	Search,
	ShieldCheck,
	Sparkles,
	X,
} from "lucide-react";
import React, { useEffect, useMemo, useState } from "react";
import type { ChatAttachment, ClearanceLevel } from "./DirectChatView";

export interface ContextItem {
	id: string;
	title: string;
	kind: "rag" | "code" | "config" | "graphs";
	summary: string;
	content: string;
	tokens: number;
	score?: number;
	hash?: string;
	clearance: ClearanceLevel;
}

export interface ShareContextModalProps {
	isOpen: boolean;
	onClose: () => void;
	onSelectContext: (attachment: ChatAttachment) => void;
	defaultClearance?: ClearanceLevel;
	contexts?: ContextItem[];
}

const AVAILABLE_CONTEXTS: ContextItem[] = [
	{
		id: "ctx-rag-mesh-spec",
		title: "SWAL Mesh Protocol & Telecom Specifications",
		kind: "rag",
		summary: "Iroh QUIC wire frame definitions, X25519 bilateral key exchange & ChaCha20-Poly1305 frame packaging.",
		content: `# SWAL Mesh Protocol Specification\n- Layer: Transport QUIC via Iroh\n- Framing: ML-DSA-65 / Ed25519 BIP39 Node ID\n- Ciphersuite: ChaCha20-Poly1305 AEAD with BLAKE3 nonce sequencing\n- Sync: Loro CRDT per-space vault storage`,
		tokens: 420,
		score: 0.96,
		hash: "9b3c4f7a1d8e2b5c6f0a3e4d5c6b7a8f9e0d1c2b3a4f5e6d7c8b9a0f1e2d3c4b",
		clearance: "Secret",
	},
	{
		id: "ctx-rag-dao-governance",
		title: "DAO Governance & Maintenance Policy Ledger",
		kind: "rag",
		summary: "Consensus thresholds, quorum rules, and multi-node maintenance lifecycle requirements.",
		content: `# DAO Maintenance Policy\n- Minimum Quorum: 60% active node validators\n- Approval Threshold: Simple majority (>50% votesFor)\n- Node Endorsement: Required for kernel upgrades and DB migration schemas`,
		tokens: 280,
		score: 0.89,
		hash: "7f8e9d0c1b2a3f4e5d6c7b8a9f0e1d2c3b4a5f6e7d8c9b0a1f2e3d4c5b6a7f8e",
		clearance: "Internal",
	},
	{
		id: "ctx-code-espacio-store",
		title: "EspacioStore::open_isolated_vault AST & Signatures",
		kind: "code",
		summary: "Rust symbol definitions for opening dedicated per-space SQLite vaults at data/db/spaces/{id}/espacio.sqlite.",
		content: `pub struct EspacioStore {\n    pool: SqlitePool,\n    space_id: String,\n    clearance_ceiling: ClearanceLevel,\n}\n\nimpl EspacioStore {\n    pub async fn open_vault(path: &Path) -> Result<Self, StorageError> {\n        // Initializes dedicated space-isolated SQLite\n    }\n}`,
		tokens: 195,
		score: 0.94,
		hash: "2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c",
		clearance: "Confidential",
	},
	{
		id: "ctx-graphs-codegraph-ast",
		title: "AST Symbol Graph: Telecom Pipeline & Gateways",
		kind: "graphs",
		summary: "Cross-file call graph mapping execute_code_tool -> clearance_gate -> chunk_assembler.",
		content: `graph TD\n  execute_code_tool --> clearance_gate\n  clearance_gate --> chunk_assembler\n  chunk_assembler --> wire_protocol_send\n  wire_protocol_send --> sqlite_vec_audit`,
		tokens: 160,
		score: 0.85,
		hash: "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b",
		clearance: "Internal",
	},
];

export const ShareContextModal: React.FC<ShareContextModalProps> = ({
	isOpen,
	onClose,
	onSelectContext,
	defaultClearance = "Secret",
	contexts = AVAILABLE_CONTEXTS,
}) => {
	const [searchQuery, setSearchQuery] = useState("");
	const [selectedKind, setSelectedKind] = useState<string>("all");
	const [selectedContext, setSelectedContext] = useState<ContextItem | null>(null);
	const [selectedClearance, setSelectedClearance] = useState<ClearanceLevel>(defaultClearance);

	useEffect(() => {
		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.key === "Escape" && isOpen) {
				onClose();
			}
		};
		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [isOpen, onClose]);

	const filteredContexts = useMemo(() => {
		return contexts.filter((item) => {
			const matchesKind =
				selectedKind === "all" || item.kind === selectedKind;
			const matchesSearch =
				item.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
				item.summary.toLowerCase().includes(searchQuery.toLowerCase());
			return matchesKind && matchesSearch;
		});
	}, [contexts, searchQuery, selectedKind]);

	if (!isOpen) return null;

	const handleConfirm = () => {
		if (!selectedContext) return;

		const attachment: ChatAttachment = {
			name: `Context: ${selectedContext.title}`,
			size: `${selectedContext.tokens} tokens`,
			type: "application/x-xavier-context",
			isContext: true,
			contextPayload: {
				kind: selectedContext.kind,
				title: selectedContext.title,
				content: selectedContext.content,
				tokens: selectedContext.tokens,
				score: selectedContext.score,
				hash: selectedContext.hash,
			},
		};

		onSelectContext(attachment);
		onClose();
	};

	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fadeIn"
			role="dialog"
			aria-modal="true"
			aria-labelledby="share-context-title"
		>
			<div className="w-full max-w-3xl max-h-[85vh] h-[680px] flex flex-col bg-slate-900 border border-emerald-500/30 rounded-2xl shadow-2xl overflow-hidden">
				{/* Top Security Banner */}
				<div className="bg-emerald-950/60 border-b border-emerald-500/20 px-4 py-2 flex items-center justify-between text-xs font-mono">
					<div className="flex items-center gap-2 text-emerald-400">
						<Brain className="w-4 h-4 text-emerald-400" />
						<span className="font-semibold">
							Xavier Context & Memory Dispatcher
						</span>
						<span className="hidden sm:inline text-emerald-500/70">
							· Bilateral P2P Sharing
						</span>
					</div>
					<div className="flex items-center gap-1.5 text-[10px] text-white/50">
						<Lock className="w-3 h-3 text-emerald-400" />
						<span>E2E Encrypted Payload</span>
					</div>
				</div>

				{/* Header */}
				<header className="px-6 py-4 bg-slate-950/70 border-b border-white/10 flex items-center justify-between">
					<div>
						<h3
							id="share-context-title"
							className="text-base font-semibold text-white"
						>
							Compartir Contexto de Memoria Xavier
						</h3>
						<p className="text-xs text-white/50 mt-0.5">
							Selecciona fragmentos RAG, símbolos del CodeGraph o especificaciones
							para vincular a la conversación bilateral.
						</p>
					</div>
					<button
						type="button"
						onClick={onClose}
						className="p-2 rounded-lg bg-white/5 border border-white/10 text-white/70 hover:text-white hover:bg-rose-500/20 hover:border-rose-500/40 hover:text-rose-300 transition-colors"
						aria-label="Cerrar selector de contexto"
					>
						<X className="w-4 h-4" />
					</button>
				</header>

				{/* Filter & Search Bar */}
				<div className="p-4 bg-slate-950/40 border-b border-white/10 flex flex-col sm:flex-row gap-3">
					<div className="relative flex-1">
						<Search className="w-4 h-4 absolute left-3 top-2.5 text-white/40" />
						<input
							type="text"
							placeholder="Buscar memorias, símbolos o especificaciones..."
							value={searchQuery}
							onChange={(e) => setSearchQuery(e.target.value)}
							className="w-full bg-slate-900 border border-white/10 rounded-xl pl-9 pr-4 py-2 text-xs text-white placeholder-white/40 focus:outline-none focus:border-emerald-500/50 font-mono"
						/>
					</div>

					{/* Category Tabs */}
					<div className="flex items-center gap-1 bg-slate-900 p-1 rounded-xl border border-white/10 text-xs">
						<button
							type="button"
							onClick={() => setSelectedKind("all")}
							className={`px-3 py-1.5 rounded-lg transition-colors ${
								selectedKind === "all"
									? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/40"
									: "text-white/60 hover:text-white"
							}`}
						>
							Todos
						</button>
						<button
							type="button"
							onClick={() => setSelectedKind("rag")}
							className={`px-3 py-1.5 rounded-lg transition-colors ${
								selectedKind === "rag"
									? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/40"
									: "text-white/60 hover:text-white"
							}`}
						>
							RAG
						</button>
						<button
							type="button"
							onClick={() => setSelectedKind("code")}
							className={`px-3 py-1.5 rounded-lg transition-colors ${
								selectedKind === "code"
									? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/40"
									: "text-white/60 hover:text-white"
							}`}
						>
							Code
						</button>
						<button
							type="button"
							onClick={() => setSelectedKind("graphs")}
							className={`px-3 py-1.5 rounded-lg transition-colors ${
								selectedKind === "graphs"
									? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/40"
									: "text-white/60 hover:text-white"
							}`}
						>
							Grafos
						</button>
					</div>
				</div>

				{/* Context List */}
				<main className="flex-1 overflow-y-auto p-4 space-y-2.5 bg-slate-950/20">
					{filteredContexts.length === 0 ? (
						<div className="h-full flex flex-col items-center justify-center text-center p-8 text-white/40 text-xs">
							<Layers className="w-8 h-8 mb-2 opacity-30" />
							<span>No se encontraron contextos que coincidan con la búsqueda.</span>
						</div>
					) : (
						filteredContexts.map((ctx) => {
							const isSelected = selectedContext?.id === ctx.id;
							return (
								<div
									key={ctx.id}
									onClick={() => setSelectedContext(ctx)}
									className={`p-4 rounded-xl border transition-all cursor-pointer flex items-start justify-between gap-3 ${
										isSelected
											? "bg-emerald-500/10 border-emerald-500/40 shadow-lg shadow-emerald-950/40"
											: "bg-slate-900/60 border-white/5 hover:border-white/20 hover:bg-slate-900/90"
									}`}
								>
									<div className="flex items-start gap-3 min-w-0">
										<div
											className={`p-2 rounded-lg flex-shrink-0 ${
												ctx.kind === "rag"
													? "bg-emerald-500/20 text-emerald-400"
													: ctx.kind === "code"
													? "bg-cyan-500/20 text-cyan-400"
													: "bg-purple-500/20 text-purple-400"
											}`}
										>
											{ctx.kind === "rag" ? (
												<Sparkles className="w-4 h-4" />
											) : ctx.kind === "code" ? (
												<FileCode className="w-4 h-4" />
											) : (
												<Layers className="w-4 h-4" />
											)}
										</div>

										<div className="min-w-0">
											<div className="flex items-center gap-2">
												<h4 className="text-xs font-semibold text-white truncate">
													{ctx.title}
												</h4>
												<span className="px-1.5 py-0.5 rounded text-[9px] uppercase font-mono font-bold bg-white/5 border border-white/10 text-white/60">
													{ctx.kind}
												</span>
											</div>
											<p className="text-[11px] text-white/60 mt-1 line-clamp-2">
												{ctx.summary}
											</p>
											<div className="flex items-center gap-3 text-[10px] text-white/40 font-mono mt-2">
												<span>{ctx.tokens} tokens</span>
												{ctx.score !== undefined && (
													<span>· Score: {(ctx.score * 100).toFixed(0)}%</span>
												)}
												<span>· Nivel base: {ctx.clearance}</span>
											</div>
										</div>
									</div>

									<div className="flex-shrink-0 flex items-center">
										<div
											className={`w-5 h-5 rounded-full border flex items-center justify-center transition-colors ${
												isSelected
													? "bg-emerald-500 border-emerald-400 text-slate-950"
													: "border-white/20 bg-black/40"
											}`}
										>
											{isSelected && <Check className="w-3.5 h-3.5 stroke-[3]" />}
										</div>
									</div>
								</div>
							);
						})
					)}
				</main>

				{/* Footer Options */}
				<footer className="px-6 py-3.5 bg-slate-950/80 border-t border-white/10 flex items-center justify-between gap-4">
					<div className="flex items-center gap-3">
						<span className="text-xs text-white/60 font-mono">
							Nivel de Autorización:
						</span>
						<select
							value={selectedClearance}
							onChange={(e) =>
								setSelectedClearance(e.target.value as ClearanceLevel)
							}
							className="bg-slate-900 border border-white/10 text-xs font-mono text-white px-2.5 py-1.5 rounded-lg focus:outline-none focus:border-emerald-500/50"
						>
							<option value="Unclassified">Unclassified</option>
							<option value="Internal">Internal</option>
							<option value="Confidential">Confidential</option>
							<option value="Secret">Secret</option>
							<option value="TopSecret">TopSecret</option>
						</select>
					</div>

					<div className="flex items-center gap-2">
						<button
							type="button"
							onClick={onClose}
							className="px-4 py-2 rounded-xl bg-white/5 border border-white/10 text-white/70 hover:text-white hover:bg-white/10 text-xs font-medium transition-colors"
						>
							Cancelar
						</button>
						<button
							type="button"
							onClick={handleConfirm}
							disabled={!selectedContext}
							className="px-4 py-2 rounded-xl bg-emerald-500 hover:bg-emerald-400 disabled:opacity-40 text-slate-950 text-xs font-semibold flex items-center gap-1.5 transition-colors"
						>
							<Sparkles className="w-3.5 h-3.5" />
							<span>Adjuntar Contexto</span>
						</button>
					</div>
				</footer>
			</div>
		</div>
	);
};

export default ShareContextModal;