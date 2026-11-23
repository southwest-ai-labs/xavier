import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ShareContextModal from "../src/components/Telecom/ShareContextModal";

describe("ShareContextModal", () => {
	it("renders nothing when isOpen is false", () => {
		const { container } = render(
			<ShareContextModal
				isOpen={false}
				onClose={vi.fn()}
				onSelectContext={vi.fn()}
			/>,
		);
		expect(container.firstChild).toBeNull();
	});

	it("renders context list, search bar, and category filters", () => {
		render(
			<ShareContextModal
				isOpen={true}
				onClose={vi.fn()}
				onSelectContext={vi.fn()}
			/>,
		);

		expect(
			screen.getByText("Compartir Contexto de Memoria Xavier"),
		).toBeInTheDocument();
		expect(
			screen.getByText("SWAL Mesh Protocol & Telecom Specifications"),
		).toBeInTheDocument();
		expect(
			screen.getByText("DAO Governance & Maintenance Policy Ledger"),
		).toBeInTheDocument();
		expect(
			screen.getByPlaceholderText(
				"Buscar memorias, símbolos o especificaciones...",
			),
		).toBeInTheDocument();
	});

	it("filters context items by category", () => {
		render(
			<ShareContextModal
				isOpen={true}
				onClose={vi.fn()}
				onSelectContext={vi.fn()}
			/>,
		);

		// Click "Code" category filter
		const codeTab = screen.getByRole("button", { name: "Code" });
		fireEvent.click(codeTab);

		expect(
			screen.getByText("EspacioStore::open_isolated_vault AST & Signatures"),
		).toBeInTheDocument();
		expect(
			screen.queryByText("DAO Governance & Maintenance Policy Ledger"),
		).not.toBeInTheDocument();
	});

	it("selects a context item and triggers onSelectContext callback", () => {
		const handleSelect = vi.fn();
		const handleClose = vi.fn();

		render(
			<ShareContextModal
				isOpen={true}
				onClose={handleClose}
				onSelectContext={handleSelect}
			/>,
		);

		// Click on first context item
		const firstItem = screen.getByText(
			"SWAL Mesh Protocol & Telecom Specifications",
		);
		fireEvent.click(firstItem);

		// Click "Adjuntar Contexto"
		const submitBtn = screen.getByRole("button", {
			name: /Adjuntar Contexto/i,
		});
		fireEvent.click(submitBtn);

		expect(handleSelect).toHaveBeenCalledWith(
			expect.objectContaining({
				isContext: true,
				type: "application/x-xavier-context",
				contextPayload: expect.objectContaining({
					kind: "rag",
					title: "SWAL Mesh Protocol & Telecom Specifications",
				}),
			}),
		);
		expect(handleClose).toHaveBeenCalledTimes(1);
	});

	it("closes on Escape key press", () => {
		const handleClose = vi.fn();
		render(
			<ShareContextModal
				isOpen={true}
				onClose={handleClose}
				onSelectContext={vi.fn()}
			/>,
		);

		fireEvent.keyDown(window, { key: "Escape" });
		expect(handleClose).toHaveBeenCalledTimes(1);
	});
});