import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import DirectChatView, {
	type DirectChatMessage,
	type DirectPeerInfo,
} from "../src/components/Telecom/DirectChatView";

describe("DirectChatView", () => {
	const customPeer: DirectPeerInfo = {
		nodeId: "node-alpha-77",
		alias: "Peer Alpha",
		online: true,
		clearanceLevel: "Secret",
		lastSeen: "Online",
	};

	const customMessages: DirectChatMessage[] = [
		{
			id: "msg-1",
			roomId: "room-test",
			senderId: "node-alpha-77",
			senderAlias: "Peer Alpha",
			content: "Hello from Peer Alpha via encrypted channel.",
			timestamp: "11:00 AM",
			clearanceLevel: "Secret",
			status: "read",
			isSelf: false,
		},
		{
			id: "msg-2",
			roomId: "room-test",
			senderId: "node-self",
			senderAlias: "Local Node",
			content: "Acknowledged. Direct session verified.",
			timestamp: "11:01 AM",
			clearanceLevel: "Confidential",
			status: "delivered",
			isSelf: true,
		},
		{
			id: "msg-3",
			roomId: "room-test",
			senderId: "node-self",
			senderAlias: "Local Node",
			content: "Sent payload pending receipt.",
			timestamp: "11:02 AM",
			clearanceLevel: "Secret",
			status: "sent",
			isSelf: true,
		},
	];

	it("renders peer header info and ChaCha20-Poly1305 security badge", () => {
		render(<DirectChatView peer={customPeer} messages={customMessages} />);

		expect(screen.getAllByText("Peer Alpha").length).toBeGreaterThanOrEqual(1);
		expect(screen.getByText("node-alpha-77")).toBeInTheDocument();
		expect(
			screen.getByText(/Online · Direct Link Active/i),
		).toBeInTheDocument();
		expect(screen.getByText("ChaCha20-Poly1305")).toBeInTheDocument();
	});

	it("displays message stream with receipts (read, delivered, sent)", () => {
		render(<DirectChatView peer={customPeer} messages={customMessages} />);

		expect(
			screen.getByText("Hello from Peer Alpha via encrypted channel."),
		).toBeInTheDocument();
		expect(
			screen.getByText("Acknowledged. Direct session verified."),
		).toBeInTheDocument();
		expect(
			screen.getByText("Sent payload pending receipt."),
		).toBeInTheDocument();

		// Check receipt status text or elements
		expect(screen.getByText("delivered")).toBeInTheDocument();
		expect(screen.getByText("sent")).toBeInTheDocument();
	});

	it("sends a direct message and invokes onSendMessage callback", () => {
		const handleSendMessage = vi.fn();
		render(
			<DirectChatView
				peer={customPeer}
				messages={customMessages}
				onSendMessage={handleSendMessage}
			/>,
		);

		const input = screen.getByPlaceholderText(
			/Send encrypted message to Peer Alpha.../i,
		);
		fireEvent.change(input, {
			target: { value: "Testing 1-to-1 message dispatch" },
		});

		const sendButton = screen.getByLabelText("Send direct message");
		fireEvent.click(sendButton);

		expect(
			screen.getByText("Testing 1-to-1 message dispatch"),
		).toBeInTheDocument();
		expect(handleSendMessage).toHaveBeenCalledWith(
			"Testing 1-to-1 message dispatch",
			expect.objectContaining({
				ephemeral: false,
				clearanceLevel: "Secret",
			}),
		);
		expect((input as HTMLInputElement).value).toBe("");
	});

	it("supports toggling off-the-record ephemeral mode", () => {
		const handleSendMessage = vi.fn();
		render(
			<DirectChatView
				peer={customPeer}
				messages={customMessages}
				onSendMessage={handleSendMessage}
			/>,
		);

		const ephemeralBtn = screen.getByLabelText(
			"Toggle off-the-record ephemeral message mode",
		);
		fireEvent.click(ephemeralBtn);

		expect(screen.getByText("OTR Ephemeral ON")).toBeInTheDocument();

		const input = screen.getByPlaceholderText(
			/Send encrypted message to Peer Alpha.../i,
		);
		fireEvent.change(input, { target: { value: "Ephemeral secret payload" } });

		const sendButton = screen.getByLabelText("Send direct message");
		fireEvent.click(sendButton);

		expect(handleSendMessage).toHaveBeenCalledWith(
			"Ephemeral secret payload",
			expect.objectContaining({
				ephemeral: true,
			}),
		);
	});

	it("allows searching and filtering conversation messages", () => {
		render(<DirectChatView peer={customPeer} messages={customMessages} />);

		const searchToggle = screen.getByLabelText("Search conversation messages");
		fireEvent.click(searchToggle);

		const searchInput = screen.getByPlaceholderText(
			"Search in encrypted chat...",
		);
		fireEvent.change(searchInput, { target: { value: "Acknowledged" } });

		expect(
			screen.getByText("Acknowledged. Direct session verified."),
		).toBeInTheDocument();
	});

	it("supports attaching files and removing attachments", () => {
		render(<DirectChatView peer={customPeer} messages={customMessages} />);

		const attachButton = screen.getByLabelText("Attach file to direct message");
		const fileInput = attachButton.parentElement?.querySelector(
			'input[type="file"]',
		) as HTMLInputElement;

		const mockFile = new File(["dummy data"], "secret_keys.json", {
			type: "application/json",
		});

		fireEvent.change(fileInput, { target: { files: [mockFile] } });

		expect(screen.getByText("secret_keys.json")).toBeInTheDocument();

		const removeFileButton = screen.getByLabelText("Remove attached file");
		fireEvent.click(removeFileButton);

		expect(screen.queryByText("secret_keys.json")).not.toBeInTheDocument();
	});

	it("handles back button callback when onBack is provided", () => {
		const handleBack = vi.fn();
		render(
			<DirectChatView
				peer={customPeer}
				messages={customMessages}
				onBack={handleBack}
			/>,
		);

		const backButton = screen.getByLabelText("Back to messages list");
		fireEvent.click(backButton);

		expect(handleBack).toHaveBeenCalledTimes(1);
	});

	it("opens FileSandboxModal when clicking on a message attachment", () => {
		const messagesWithAttachment: DirectChatMessage[] = [
			{
				id: "msg-att-1",
				roomId: "room-test",
				senderId: "node-alpha-77",
				senderAlias: "Peer Alpha",
				content: "Sharing sensitive blueprint.",
				timestamp: "11:05 AM",
				clearanceLevel: "Secret",
				status: "delivered",
				isSelf: false,
				attachment: {
					name: "network_mesh_topology.png",
					size: "256 KB",
					type: "image/png",
					url: "blob:http://localhost/topology",
				},
			},
		];

		render(
			<DirectChatView peer={customPeer} messages={messagesWithAttachment} />,
		);

		expect(screen.getByText("network_mesh_topology.png")).toBeInTheDocument();
		const sandboxBtn = screen.getByText("Abrir Sandbox");
		fireEvent.click(sandboxBtn);

		expect(
			screen.getByText("Xavier Secure Sandbox Environment"),
		).toBeInTheDocument();
	});

	it("opens ShareContextModal when clicking Xavier RAG context button", () => {
		render(<DirectChatView peer={customPeer} messages={customMessages} />);

		const attachContextBtn = screen.getByLabelText(
			"Attach Xavier RAG context",
		);
		fireEvent.click(attachContextBtn);

		expect(
			screen.getByText("Compartir Contexto de Memoria Xavier"),
		).toBeInTheDocument();
	});

	it("opens Bilateral Mesh Vault Governance modal when clicking the vault header pill", () => {
		render(
			<DirectChatView
				roomId="room-beta-direct"
				peer={customPeer}
				messages={customMessages}
			/>,
		);

		const vaultPill = screen.getByTitle(
			"Ver Gobernanza de Bóveda P2P Mesh (espacio.sqlite)",
		);
		fireEvent.click(vaultPill);

		expect(
			screen.getByText("Gobernanza de Bóveda P2P Mesh"),
		).toBeInTheDocument();
		expect(
			screen.getByText("data/db/spaces/room-beta-direct/espacio.sqlite"),
		).toBeInTheDocument();
	});
});
