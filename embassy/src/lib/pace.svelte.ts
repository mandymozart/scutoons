// The signed-in person's pace (owner 2026-10-05: liaisons work at their own pace): loaded from /embassy/me once the
// session is there. `value: null` with `available` = not onboarded yet → the layout shows the onboarding instead of the
// page. Without the API's pace table (migration 2026-10-05-embassy-pace not run) `available` is false and the app works
// as before.
import type { EmbassyPace } from '@scutoons/api';
import { authApi, embassy } from '$lib/api.ts';

export const pace = $state<{ loaded: boolean; available: boolean; value: EmbassyPace | null }>({ loaded: false, available: false, value: null });

export async function loadPace() {
	try {
		const me = await embassy.me();
		pace.available = me.pace !== undefined;
		pace.value = me.pace ?? null;
	} catch {
		pace.available = false;
	} finally {
		pace.loaded = true;
	}
}

/**
 * Saves the first name the person signs with (the account's `display_name`, at most 32 characters) and reloads the
 * session, so `session.user.display_name` – and the agent's "Writing:" – has it. Nothing to save → nothing happens.
 */
export async function saveName(session: { user: { display_name?: string | null } | null; loadMe: () => Promise<unknown> }, value: string) {
	const name = value.trim().slice(0, 32);
	if (name === (session.user?.display_name ?? '').trim()) return;
	await authApi.updateMe({ display_name: name || null });
	await session.loadMe();
}

/** the onboarding / Today saved a new pace */
export function setPace(value: EmbassyPace) {
	pace.value = value;
	pace.available = true;
}
