// The creator's resume state per device (server docs/API.md "Resume across devices", #838). The state is the creator's own
// JSON: the server checks only that it is an object and its size, the creator's reader the inside – so it stays `unknown` here.
import type { ApiClient } from './client.ts';

export interface DeviceSession {
	device: string;
	/** a short label of the device: browser · OS */
	name: string;
	state: unknown;
	updated_at: string | null;
}

export interface MemberSession {
	/** false: the server has no sessions yet (before its migration) */
	available: boolean;
	/** this device's row */
	mine: DeviceSession | null;
	/** the newest row of another device */
	other: DeviceSession | null;
}

export function createDeviceSessionApi(api: ApiClient) {
	return {
		mine: (device: string) => api.get<{ status: string } & MemberSession>(`/me/session?device=${encodeURIComponent(device)}`),
		save: (body: { device: string; name: string; state: unknown }) => api.put<{ status: string; session: DeviceSession }>('/me/session', body)
	};
}

export type DeviceSessionApi = ReturnType<typeof createDeviceSessionApi>;
