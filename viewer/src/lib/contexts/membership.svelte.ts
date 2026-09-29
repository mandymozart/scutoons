import { setContext, getContext } from 'svelte';
import { ApiError } from '$lib/api/client.ts';
import { checkAccess, type AccessResult } from '$lib/api/qr.service.ts';
import type { SessionContext } from './session.svelte.ts';

const MEMBERSHIP_KEY = Symbol('membership');
/** device access from a scanned code (no account needed) */
const ACCESS_KEY = 'scutoons_access';
/** legacy device-local membership (pre-accounts) → migrated to ACCESS_KEY */
const LEGACY_KEY = 'scutoons_membership';
const RECHECK_MS = 6 * 60 * 60 * 1000;
const CATALOG = 'catalog';

export interface DeviceAccess {
	code: string;
	permissions: string[];
	expires_at: string | null;
	checked_at: string;
	/** server-signed token, kept for a future server-side content gate */
	access_token?: string;
}

/** ISO 8601 or SQL `YYYY-MM-DD HH:MM:SS` (UTC) */
function isExpired(value: string | null | undefined): boolean {
	if (!value) return false;
	const t = Date.parse(/[TZ+]/.test(value.slice(10)) ? value : value.replace(' ', 'T') + 'Z');
	return !Number.isNaN(t) && Date.now() > t;
}

export type MembershipContext = ReturnType<typeof createMembershipContext>;

/**
 * Access (final model 2026-09-30): chapters are unlocked by
 *  - device access: a scanned code whose type grants `catalog` (`/access/`, stored in `scutoons_access`,
 *    works even if someone else owns the code, survives logout), or
 *  - the logged-in account owning such a code (`/me/ access.permissions`).
 * Which code type grants what is decided by the server's permissions map — nothing is hardcoded here.
 */
export const createMembershipContext = (session: SessionContext) => {
	let device = $state<DeviceAccess | null>(null);
	/** code whose device access was dropped because the server says revoked / expired (one-time notice) */
	let invalidatedCode = $state<string | null>(null);

	const deviceValid = $derived(
		!!device && device.permissions.includes(CATALOG) && !isExpired(device.expires_at)
	);
	const isValid = $derived(deviceValid || session.hasAccess);
	/** token for a future server-side gate: the account's when logged in, else the device's */
	const accessToken = $derived<string | null>(
		(session.hasAccess ? (session.me?.access_token ?? session.me?.access?.access_token) : null) ??
			device?.access_token ??
			null
	);

	function read<T>(key: string): T | null {
		try {
			const raw = localStorage.getItem(key);
			return raw ? (JSON.parse(raw) as T) : null;
		} catch {
			return null;
		}
	}

	function store(value: DeviceAccess | null) {
		device = value;
		try {
			if (value) localStorage.setItem(ACCESS_KEY, JSON.stringify(value));
			else localStorage.removeItem(ACCESS_KEY);
		} catch {
			/* storage unavailable */
		}
	}

	function fromResult(result: AccessResult): DeviceAccess {
		return {
			code: result.code,
			permissions: result.permissions ?? [],
			expires_at: result.expires_at ?? null,
			checked_at: new Date().toISOString(),
			access_token: result.access_token
		};
	}

	/**
	 * Ask the server what a code grants. Grants device access when it includes `catalog`
	 * (an existing device access is only replaced by one that unlocks). Throws ApiError (404, 410, 429, network).
	 */
	async function grantFromCode(code: string): Promise<AccessResult> {
		const result = await checkAccess(code);
		if (result.permissions?.includes(CATALOG)) store(fromResult(result));
		return result;
	}

	/** Re-check the stored device code now and then (expiry, revocation); refreshes the access token. */
	async function recheck(force = false) {
		if (!device) return;
		const age = Date.now() - (Date.parse(device.checked_at) || 0);
		if (!force && age < RECHECK_MS && !isExpired(device.expires_at)) return;
		try {
			const result = await checkAccess(device.code);
			store(fromResult(result));
		} catch (e) {
			// revoked / expired / gone → drop (410 with a one-time notice); network / rate limit → keep, retry later
			if (e instanceof ApiError && (e.status === 404 || e.status === 410)) {
				if (e.status === 410) invalidatedCode = device.code;
				store(null);
			}
		}
	}

	/** Pre-accounts `scutoons_membership` → device access via `/access/` (silent). */
	async function migrateLegacy() {
		const legacy = read<{ code?: string }>(LEGACY_KEY);
		if (!legacy) return;
		const clear = () => {
			try {
				localStorage.removeItem(LEGACY_KEY);
				localStorage.removeItem('scutoons_membership_prompted');
			} catch {
				/* storage unavailable */
			}
		};
		if (!legacy.code) return clear();
		try {
			await grantFromCode(legacy.code);
			clear();
		} catch (e) {
			if (e instanceof ApiError && e.status !== 0 && e.status !== 429) clear();
		}
	}

	async function init() {
		device = read<DeviceAccess>(ACCESS_KEY);
		await migrateLegacy();
		await recheck();
	}

	return {
		get isValid() {
			return isValid;
		},
		get device() {
			return device;
		},
		get deviceValid() {
			return deviceValid;
		},
		get accessToken() {
			return accessToken;
		},
		get invalidatedCode() {
			return invalidatedCode;
		},
		dismissInvalidated() {
			invalidatedCode = null;
		},
		grantFromCode,
		recheck,
		init
	};
};

export const setMembershipContext = (session: SessionContext) => {
	const context = createMembershipContext(session);
	setContext(MEMBERSHIP_KEY, context);
	return context;
};

export const getMembershipContext = () => {
	return getContext<MembershipContext>(MEMBERSHIP_KEY);
};
