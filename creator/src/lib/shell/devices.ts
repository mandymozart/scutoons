// Resume across devices: the whole state this browser keeps for the member – open items with their workspaces, the
// workspace each kind shows, the panels of every workspace, the views, full screen, the dashboard, scroll – as one
// document sent to the server per device; another device's newer one is offered once. Pure: test/devices.test.ts.
import { isGone } from './address.ts';
import { readChosen } from './layouts.ts';
import { NO_SESSION, readSession, type Session } from './resume.ts';
import { readViews, type ViewSettings } from './settings.ts';
import { EMPTY, readTabs, type TabsState } from './tabs.ts';

/** this browser's device: its id, and the other device's state already offered (`device@time`) */
export const DEVICE_KEY = 'sc-creator-device';
/** the panels' workspaces of the creator in the kit's store (workspace.svelte) */
export const HOST_PREFIX = 'creator-host-';

export interface WindowSize {
	w: number;
	h: number;
}

export interface DeviceState {
	v: 1;
	/** the window the panels were placed in (CSS px, as the panels' places) */
	window: WindowSize;
	tabs: TabsState;
	/** the workspace each kind shows */
	chosen: Record<string, string>;
	/** per panels' workspace (`creator-host-<id>`) its layout { places, tabs, stage } – the kit's reader checks the inside */
	panels: Record<string, Record<string, unknown>>;
	views: Record<string, ViewSettings>;
	session: Session;
}

export interface DeviceMeta {
	id: string;
	/** the other device's state offered already: `device@updated_at` */
	seen: string | null;
}

export interface DeviceRow {
	device: string;
	name: string;
	state: unknown;
	updated_at: string | null;
}

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const ID = /^[A-Za-z0-9_-]{8,64}$/;

/** a new device id: 22 url-safe characters */
export function newDeviceId(random: (n: number) => Uint8Array): string {
	const abc = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';
	return [...random(22)].map((b) => abc[b & 63]).join('');
}

export function readMeta(raw: unknown, fresh: () => string): DeviceMeta {
	const v = isRecord(raw) ? raw : {};
	return { id: typeof v.id === 'string' && ID.test(v.id) ? v.id : fresh(), seen: typeof v.seen === 'string' ? v.seen : null };
}

/** a short name for the device from the browser's user agent: browser · OS */
export function deviceName(ua: string): string {
	const browser = /Edg\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : /Firefox\//.test(ua) ? 'Firefox' : /Chrome\//.test(ua) ? 'Chrome' : /Safari\//.test(ua) ? 'Safari' : 'Browser';
	const os = /iPad|iPhone/.test(ua) ? 'iOS' : /Android/.test(ua) ? 'Android' : /Windows/.test(ua) ? 'Windows' : /Mac OS X|Macintosh/.test(ua) ? 'macOS' : /CrOS/.test(ua) ? 'ChromeOS' : /Linux/.test(ua) ? 'Linux' : '';
	return os ? `${browser} · ${os}` : browser;
}

function readWindow(v: unknown): WindowSize | null {
	if (!isRecord(v)) return null;
	const { w, h } = v;
	return typeof w === 'number' && typeof h === 'number' && w > 0 && h > 0 && Number.isFinite(w) && Number.isFinite(h) ? { w, h } : null;
}

/** the creator's panels' layouts out of the kit's store (other editors' workspaces stay out) */
export function hostPanels(stored: unknown): Record<string, Record<string, unknown>> {
	if (!isRecord(stored)) return {};
	return Object.fromEntries(Object.entries(stored).filter((e): e is [string, Record<string, unknown>] => e[0].startsWith(HOST_PREFIX) && isRecord(e[1])));
}

/** a state sent or received, checked: anything broken reads as null */
export function readDeviceState(raw: unknown): DeviceState | null {
	if (!isRecord(raw) || raw.v !== 1) return null;
	const window = readWindow(raw.window);
	if (!window) return null;
	return {
		v: 1,
		window,
		tabs: readTabs(raw.tabs, isGone),
		chosen: readChosen(raw.chosen),
		panels: hostPanels(raw.panels),
		views: readViews(raw.views),
		session: isRecord(raw.session) ? readSession(raw.session) : NO_SESSION
	};
}

/** nothing open and nothing placed: a browser starting fresh */
export const isBlank = (s: Pick<DeviceState, 'tabs' | 'panels'>): boolean => !s.tabs.tabs.length && !Object.keys(s.panels).length;

export const NO_DEVICE_STATE: DeviceState = { v: 1, window: { w: 1, h: 1 }, tabs: EMPTY, chosen: {}, panels: {}, views: {}, session: NO_SESSION };

const time = (iso: string | null | undefined): number => {
	const t = iso ? Date.parse(iso) : NaN;
	return Number.isFinite(t) ? t : 0;
};

export const seenKey = (row: DeviceRow): string => `${row.device}@${row.updated_at ?? ''}`;

export type Resumption = { kind: 'none' } | { kind: 'own'; state: DeviceState } | { kind: 'offer'; row: DeviceRow; state: DeviceState };

/**
 * What the start does with the server's rows. This browser's own state comes first: kept here, it stays as it is; lost
 * here (a cleared browser) while the server has it, it comes back silently. Another device's state newer than this
 * device's last change (`localAt`, ms; the server's row too) is offered – once: not when it was offered already.
 */
export function resumption(local: DeviceState, localAt: number, mine: DeviceRow | null, other: DeviceRow | null, seen: string | null): Resumption {
	const own = mine ? readDeviceState(mine.state) : null;
	const ownAt = Math.max(localAt, time(mine?.updated_at));
	const theirs = other ? readDeviceState(other.state) : null;
	if (other && theirs && !isBlank(theirs) && time(other.updated_at) > ownAt && seenKey(other) !== seen) return { kind: 'offer', row: other, state: theirs };
	if (own && isBlank(local) && !isBlank(own)) return { kind: 'own', state: own };
	return { kind: 'none' };
}

interface PlaceLike {
	corner?: unknown;
	dx?: unknown;
	dy?: unknown;
	size?: { w?: unknown; h?: unknown };
}

const num = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

/**
 * One place moved from a window of size `from` into one of size `to`: its distance from its corner scales with the
 * window, its size shrinks to the window, and it stays inside – the floats' clamp (`clampInto`) taken in the place's
 * own corner coordinates; the kit clamps again when it lays the panels out.
 */
export function fitPlace<P extends PlaceLike>(place: P, from: WindowSize, to: WindowSize): P {
	if (!num(place.dx) || !num(place.dy)) return place;
	const size = place.size && num(place.size.w) && num(place.size.h) ? { w: Math.min(place.size.w, to.w), h: Math.min(place.size.h, to.h) } : null;
	const clamp = (v: number, room: number) => Math.round(Math.max(0, Math.min(v, room)));
	const dx = clamp((place.dx * to.w) / from.w, to.w - (size?.w ?? 0));
	const dy = clamp((place.dy * to.h) / from.h, to.h - (size?.h ?? 0));
	return { ...place, dx, dy, ...(size ? { size: { ...place.size, ...size } } : {}) };
}

/** every panel of every workspace fitted into this window (the same size: unchanged) */
export function fitPanels(panels: DeviceState['panels'], from: WindowSize, to: WindowSize): DeviceState['panels'] {
	if (from.w === to.w && from.h === to.h) return panels;
	const out: DeviceState['panels'] = {};
	for (const [ws, layout] of Object.entries(panels)) {
		const places = isRecord(layout.places) ? layout.places : null;
		out[ws] = places ? { ...layout, places: Object.fromEntries(Object.entries(places).map(([id, p]) => [id, isRecord(p) ? fitPlace(p as PlaceLike, from, to) : p])) } : layout;
	}
	return out;
}

/** the kit's store with the creator's workspaces replaced by `panels` (other editors' kept) */
export function withHostPanels(stored: unknown, panels: DeviceState['panels']): Record<string, unknown> {
	const rest = isRecord(stored) ? Object.fromEntries(Object.entries(stored).filter(([k]) => !k.startsWith(HOST_PREFIX))) : {};
	return { ...rest, ...panels };
}

/** when another device's state was left: the time today, else the day and the time */
export function whenLabel(iso: string, now: Date, locale: string): string {
	const at = new Date(iso);
	if (!Number.isFinite(at.getTime())) return '';
	const time = at.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' });
	if (at.toDateString() === now.toDateString()) return time;
	return `${at.toLocaleDateString(locale, { weekday: 'short', day: 'numeric', month: 'short' })} ${time}`;
}
