import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import NotificationsDropdown, { Notification } from "../src/components/NotificationsDropdown";

describe("Notifications Browser Compatibility & Functionality", () => {
	const mockNotifications: Notification[] = [
		{
			id: "notif-env-1",
			islandId: "system",
			title: "Env Token Notification",
			body: "Fetched using environment token.",
			timestamp: new Date().toISOString(),
			read: false,
			severity: "info",
		},
		{
			id: "notif-env-2",
			islandId: "errors",
			title: "Error Notification",
			body: "System error occurred.",
			timestamp: new Date().toISOString(),
			read: false,
			severity: "error",
		},
	];

	const originalFetch = global.fetch;

	beforeEach(() => {
		vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
		// Mock window.__TAURI_INTERNALS__ absent by default (Browser mode)
		// @ts-ignore
		delete window.__TAURI_INTERNALS__;

		global.fetch = vi.fn().mockImplementation((url: string) => {
			if (url.includes("/notifications")) {
				return Promise.resolve({
					ok: true,
					status: 200,
					json: () => Promise.resolve(mockNotifications),
				} as Response);
			}
			return Promise.resolve({ ok: true, json: () => Promise.resolve({}) } as Response);
		});
	});

	afterEach(() => {
		vi.useRealTimers();
		global.fetch = originalFetch;
		vi.restoreAllMocks();
	});

	it("(d) skeletons mientras loading en NotificationsDropdown", async () => {
		let resolveFetch: (value: any) => void;
		global.fetch = vi.fn().mockImplementation(
			() =>
				new Promise((resolve) => {
					resolveFetch = resolve;
				}),
		);

		const { container } = render(
			<NotificationsDropdown onClose={() => {}} />,
		);

		// Verify skeletons (animate-pulse elements) are visible during loading
		const skeletons = container.querySelectorAll(".animate-pulse");
		expect(skeletons.length).toBeGreaterThan(0);

		// Resolve fetch
		await act(async () => {
			resolveFetch({
				ok: true,
				status: 200,
				json: () => Promise.resolve(mockNotifications),
			});
		});

		await waitFor(() => {
			expect(screen.getByText("Env Token Notification")).toBeInTheDocument();
		});
	});

	it("(g) no crash transformCallback en browser environment", async () => {
		// Verify rendering in pure browser env without window.__TAURI_INTERNALS__ does not throw transformCallback error
		expect(() => {
			render(
				<NotificationsDropdown onClose={() => {}} />,
			);
		}).not.toThrow();

		await waitFor(() => {
			expect(screen.getByText("Env Token Notification")).toBeInTheDocument();
		});
	});

	it("renders clickable commit details link in notification body", async () => {
		const updateNotif: Notification[] = [
			{
				id: "notif-update-1",
				islandId: "system",
				title: "Xavier actualizado silenciosamente (fe4125c)",
				body: "Se aplicaron las siguientes mejoras desde main:\n• feat(ui): responsive panel\nDetalles: https://github.com/southwest-ai-labs/xavier/commit/fe4125ca7",
				timestamp: new Date().toISOString(),
				read: false,
				severity: "success",
			},
		];

		global.fetch = vi.fn().mockImplementation((url: string) => {
			if (url.includes("/notifications")) {
				return Promise.resolve({
					ok: true,
					status: 200,
					json: () => Promise.resolve(updateNotif),
				} as Response);
			}
			return Promise.resolve({ ok: true, json: () => Promise.resolve({}) } as Response);
		});

		render(<NotificationsDropdown onClose={() => {}} />);

		await waitFor(() => {
			expect(screen.getByText("Xavier actualizado silenciosamente (fe4125c)")).toBeInTheDocument();
		});

		const link = screen.getByRole("link", { name: /ver detalles del commit/i });
		expect(link).toBeInTheDocument();
		expect(link).toHaveAttribute("href", "https://github.com/southwest-ai-labs/xavier/commit/fe4125ca7");
		expect(link).toHaveAttribute("target", "_blank");
	});
});
