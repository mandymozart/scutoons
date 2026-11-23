// Reopening resumes like a hibernated computer (research/redesign/lap/workspace-thoughts.md points 54, 55): what the
// open tabs (tabs.ts) and the panels' layouts (the kit's floats) don't keep – the dashboard, each workspace's view full
// screen, scroll positions – kept in this browser per member, and the start: the saved tabs merged with the URL opened.
// Pure: test/resume.test.ts.
import { parseEntityKey } from '@scutoons/content';
import { AREAS, isGone, kindOf } from './address.ts';
import { close, EMPTY, readTabs, visit, type TabsState } from './tabs.ts';

/** where this browser keeps it; a new shape takes a new version and the old key is simply not read */
export const RESUME_KEY = 'sc-creator-resume.v1';

export type DashboardState = false | 'cards';

/** a scroll position: how far down, and – inside a panel – the scroller's child path from the panel */
export interface Scroll {
	top: number;
	path?: number[];
}

/** one member's state past the tabs */
export interface Session {
	dashboard: DashboardState;
	/** per workspace (the floats' id): the view full screen */
	full: Record<string, string>;
	/** `stage:<address>` (the stage page under an item) and `panel:<workspace>/<panel>/<tab>` */
	scroll: Record<string, Scroll>;
}

export interface ResumeStore extends Session {
	v: 1;
	/** whose state this is (null: not known yet – the first visit, or signed out) */
	member: string | null;
	/** others who used this browser: their tabs and state, back when they sign in again */
	others: Record<string, Session & { tabs: TabsState }>;
}

export const NO_SESSION: Session = { dashboard: false, full: {}, scroll: {} };
export const NO_RESUME: ResumeStore = { v: 1, member: null, ...NO_SESSION, others: {} };

/** scroll positions kept per member; the oldest go first */
export const MAX_SCROLLS = 60;
/** other members kept in one browser */
export const MAX_OTHERS = 6;

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);

const readDashboard = (v: unknown): DashboardState => (v === 'cards' ? v : false);

function readScroll(v: unknown): Scroll | null {
	if (!isRecord(v) || typeof v.top !== 'number' || !Number.isFinite(v.top) || v.top <= 0) return null;
	const path = Array.isArray(v.path) && v.path.length <= 32 && v.path.every((n) => Number.isInteger(n) && n >= 0) ? (v.path as number[]) : undefined;
	return { top: Math.round(v.top), ...(path ? { path } : {}) };
}

export function readSession(v: Record<string, unknown>): Session {
	const full: Record<string, string> = {};
	if (isRecord(v.full)) for (const [ws, tab] of Object.entries(v.full)) if (typeof tab === 'string' && tab) full[ws] = tab;
	const scroll: Record<string, Scroll> = {};
	if (isRecord(v.scroll))
		for (const [key, s] of Object.entries(v.scroll).slice(-MAX_SCROLLS)) {
			const read = (key.startsWith('stage:') || key.startsWith('panel:')) && readScroll(s);
			if (read) scroll[key] = read;
		}
	return { dashboard: readDashboard(v.dashboard), full, scroll };
}

/** what the browser kept, checked: another version or a broken store reads as nothing kept */
export function readResume(raw: unknown): ResumeStore {
	if (!isRecord(raw) || raw.v !== 1) return NO_RESUME;
	const others: ResumeStore['others'] = {};
	if (isRecord(raw.others))
		for (const [id, o] of Object.entries(raw.others).slice(-MAX_OTHERS)) if (id && isRecord(o)) others[id] = { ...readSession(o), tabs: readTabs(o.tabs, isGone) };
	return { v: 1, member: typeof raw.member === 'string' && raw.member ? raw.member : null, ...readSession(raw), others };
}

/** where the creator was opened: the URL's item (null: Home) and its page */
export interface Opened {
	address: string | null;
	href: string;
}

export interface Start {
	tabs: TabsState;
	/** the page to go to now (the resumed front tab), or null: stay on the URL opened */
	go: string | null;
	dashboard: DashboardState;
}

/**
 * The start: the kept tabs (none when `reopen` is off) and the URL the creator was opened at. Home opened (a new start,
 * the browser reopened): the front tab comes back with the dashboard as it was – nothing in front, the dashboard. The
 * front tab's own page (a reload): it stays, the dashboard as it was. Any other item (a link opened): it is the front tab,
 * the others stay behind it, no dashboard.
 */
export function startOf(kept: TabsState, session: Session, reopen: boolean, opened: Opened): Start {
	const tabs = reopen ? kept : EMPTY;
	const front = tabs.tabs.find((t) => t.address === tabs.front) ?? null;
	if (!opened.address) return front ? { tabs, go: front.href, dashboard: session.dashboard } : { tabs: { ...tabs, front: null }, go: null, dashboard: 'cards' };
	if (front?.address === opened.address) return { tabs: visit(tabs, opened.address, opened.href), go: null, dashboard: session.dashboard };
	return { tabs: visit(tabs, opened.address, opened.href), go: null, dashboard: false };
}

/**
 * Someone else signed in on this browser: the one before keeps their tabs and state among the others, the new one gets
 * theirs back (a new member: nothing). The same member, or the first known one: unchanged.
 */
export function switchMember(store: ResumeStore, tabs: TabsState, member: string): { store: ResumeStore; tabs: TabsState } {
	if (store.member === member) return { store, tabs };
	if (store.member === null) return { store: { ...store, member }, tabs };
	const { [member]: mine, ...rest } = store.others;
	const kept = { dashboard: store.dashboard, full: store.full, scroll: store.scroll, tabs };
	const others = Object.fromEntries(Object.entries({ ...rest, [store.member]: kept }).slice(-MAX_OTHERS));
	const session = mine ?? { ...NO_SESSION, tabs: EMPTY };
	return { store: { v: 1, member, dashboard: session.dashboard, full: session.full, scroll: session.scroll, others }, tabs: session.tabs };
}

/** a workspace's view full screen, or none */
export function withFull(store: ResumeStore, workspace: string, tab: string | null): ResumeStore {
	if ((store.full[workspace] ?? null) === tab) return store;
	const { [workspace]: _was, ...full } = store.full;
	return { ...store, full: tab ? { ...full, [workspace]: tab } : full };
}

/** a scroll position (0 forgets it); the newest last, the oldest beyond the limit go */
export function withScroll(store: ResumeStore, key: string, scroll: Scroll): ResumeStore {
	const was = store.scroll[key];
	const top = Math.round(scroll.top);
	if (was && was.top === top && String(was.path) === String(scroll.path)) return store;
	const { [key]: _was, ...rest } = store.scroll;
	if (top <= 0) return { ...store, scroll: rest };
	const entries = Object.entries({ ...rest, [key]: { top, ...(scroll.path ? { path: scroll.path } : {}) } });
	return { ...store, scroll: Object.fromEntries(entries.slice(-MAX_SCROLLS)) };
}

export const stageKey = (address: string | null): string => `stage:${address ?? 'home'}`;
export const panelKey = (workspace: string, panel: string, tab: string): string => `panel:${workspace}/${panel}/${tab}`;

/** the areas' own addresses: lists, always there */
const AREA_ADDRESSES = new Set<string>(Object.values(AREAS));

/**
 * Whether an opened item still exists and is the member's to open: `lists` are the item lists by kind word (as the
 * member may read them; a list not loaded is missing), `toons` the member's toons (null: not loaded). Unknown
 * (nothing to check it against): null – it stays.
 */
export function stillThere(address: string, lists: Readonly<Record<string, readonly string[] | undefined>>, toons: readonly string[] | null): boolean | null {
	if (AREA_ADDRESSES.has(address) || address.startsWith('catalog:')) return true;
	const kind = kindOf(address);
	if (kind === 'toon' || kind === 'season' || kind === 'chapter' || kind === 'storyboard') {
		const toon = parseEntityKey(address)?.toon;
		return toons && toon ? toons.includes(toon) : null;
	}
	const list = lists[kind];
	return list ? list.includes(address) : null;
}

/** the tabs without those gone (a front one hands over as a closed tab does) */
export function withoutGone(tabs: TabsState, gone: readonly string[]): TabsState {
	return gone.reduce((state, address) => close(state, address), tabs);
}
