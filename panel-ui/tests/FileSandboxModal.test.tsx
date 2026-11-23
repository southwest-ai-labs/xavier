import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import FileSandboxModal from "../src/components/Telecom/FileSandboxModal";
import type { ChatAttachment } from "../src/components/Telecom/DirectChatView";

describe("FileSandboxModal", () => {
	const mockContextAttachment: ChatAttachment = {
		name: "SWAL Mesh Protocol Spec",
		size: "420 tokens",
		type: "application/x-xavier-context",
		isContext: true,
		contextPayload: {
			kind: "rag",
			title: "SWAL Mesh Protocol Spec",
			content: "# Wire Framing\nIroh QUIC with ChaCha20-Poly1305",
			tokens: 420,
			score: 0.95,
			hash: "abc123hash999",
		},
	};

	it("renders nothing when isOpen is false", () => {
		const { container } = render(
			<FileSandboxModal
				isOpen={false}
				onClose={vi.fn()}
				attachment={mockContextAttachment}
			/>,
		);
		expect(container.firstChild).toBeNull();
	});

	it("renders sandbox environment header and security guarantees", () => {
		render(
			<FileSandboxModal
				isOpen={true}
				onClose={vi.fn()}
				attachment={mockContextAttachment}
				clearanceLevel="Secret"
			/>,
		);

		expect(
			screen.getByText("Xavier Secure Sandbox Environment"),
		).toBeInTheDocument();
		expect(
			screen.getByText(/Isolated Runtime · Zero Host Contamination/i),
		).toBeInTheDocument();
		expect(screen.getByText("Secret")).toBeInTheDocument();
	});

	it("renders Xavier RAG context details, hash, tokens, and content", () => {
		render(
			<FileSandboxModal
				isOpen={true}
				onClose={vi.fn()}
				attachment={mockContextAttachment}
			/>,
		);

		expect(
			screen.getAllByText("SWAL Mesh Protocol Spec").length,
		).toBeGreaterThanOrEqual(1);
		expect(screen.getByText("Kind: rag")).toBeInTheDocument();
		expect(screen.getAllByText(/420 tokens/).length).toBeGreaterThanOrEqual(1);
		expect(screen.getByText("abc123hash999")).toBeInTheDocument();
		expect(screen.getByText(/Wire Framing/)).toBeInTheDocument();
		expect(
			screen.getByText(/Iroh QUIC with ChaCha20-Poly1305/),
		).toBeInTheDocument();
	});

	it("triggers import to vault callback when clicking Anexar a Bóveda Mesh", () => {
		const handleImport = vi.fn();
		render(
			<FileSandboxModal
				isOpen={true}
				onClose={vi.fn()}
				attachment={mockContextAttachment}
				onImportToVault={handleImport}
			/>,
		);

		const importBtn = screen.getByTitle(
			"Import into local P2P Mesh Vault (espacio.sqlite)",
		);
		fireEvent.click(importBtn);

		expect(handleImport).toHaveBeenCalledWith(mockContextAttachment);
		expect(screen.getByText("Anexado a Bóveda")).toBeInTheDocument();
	});

	it("renders safe image preview for image attachments", () => {
		const imgAttachment: ChatAttachment = {
			name: "diagram.png",
			size: "128 KB",
			type: "image/png",
			url: "blob:http://localhost/diagram-uuid",
		};

		render(
			<FileSandboxModal
				isOpen={true}
				onClose={vi.fn()}
				attachment={imgAttachment}
			/>,
		);

		const img = screen.getByRole("img", { name: "diagram.png" });
		expect(img).toHaveAttribute("src", "blob:http://localhost/diagram-uuid");
	});

	it("renders sandboxed iframe for PDF documents", () => {
		const pdfAttachment: ChatAttachment = {
			name: "whitepaper.pdf",
			size: "2.4 MB",
			type: "application/pdf",
			url: "blob:http://localhost/pdf-uuid",
		};

		render(
			<FileSandboxModal
				isOpen={true}
				onClose={vi.fn()}
				attachment={pdfAttachment}
			/>,
		);

		const iframe = screen.getByTitle("whitepaper.pdf");
		expect(iframe).toHaveAttribute("sandbox", "allow-scripts");
		expect(iframe).toHaveAttribute("src", "blob:http://localhost/pdf-uuid");
	});

	it("calls onClose when clicking close button", () => {
		const handleClose = vi.fn();
		render(
			<FileSandboxModal
				isOpen={true}
				onClose={handleClose}
				attachment={mockContextAttachment}
			/>,
		);

		const closeBtn = screen.getByLabelText("Close sandbox viewer");
		fireEvent.click(closeBtn);
		expect(handleClose).toHaveBeenCalledTimes(1);
	});

	it("closes on Escape key press", () => {
		const handleClose = vi.fn();
		render(
			<FileSandboxModal
				isOpen={true}
				onClose={handleClose}
				attachment={mockContextAttachment}
			/>,
		);

		fireEvent.keyDown(window, { key: "Escape" });
		expect(handleClose).toHaveBeenCalledTimes(1);
	});
});